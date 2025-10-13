use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use rustyline::completion::{Candidate, Completer, Pair};
use rustyline::config::BellStyle;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::History;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Editor, Helper};
use rustyline::{Context, Result};

struct AutoCompleter {
    builtins: Vec<String>,
    executables: HashMap<String, String>,
}

impl AutoCompleter {
    fn new() -> Self {
        let mut executables_store = HashMap::new();
        let path_var = env::var("PATH");

        if path_var.is_ok() {
            let paths = path_var.expect("cannot get paths");
            for dir in paths.split(":") {
                if !std::path::Path::new(dir).exists() {
                    continue;
                }
                for entry in std::fs::read_dir(dir).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    let path_str = path.to_str().unwrap();
                    let name = path.file_name().unwrap().to_str().unwrap();
                    if executables_store.contains_key(name) {
                        continue;
                    }
                    executables_store.insert(name.to_string(), path_str.to_string());
                }
            }
        }

        Self {
            builtins: vec![
                "echo".to_string(),
                "exit".to_string(),
                "type".to_string(),
                "pwd".to_string(),
            ],
            executables: executables_store,
        }
    }

    fn autocomplete(&self, incomplete_cmd: &str) -> Vec<Pair> {
        let mut matches = Vec::new();

        for builtin in &self.builtins {
            if builtin.starts_with(&incomplete_cmd) {
                matches.push(Pair {
                    display: format!("{}", builtin.clone()),
                    replacement: format!("{} ", builtin.clone()),
                });
            }
        }

        for program in self.executables.keys() {
            if program.starts_with(&incomplete_cmd) {
                matches.push(Pair {
                    display: format!("{}", program.clone()),
                    replacement: format!("{} ", program.clone()),
                });
            }
        }

        matches
    }
}

struct MyHelper {
    completer: AutoCompleter,
}

impl Helper for MyHelper {}

impl Hinter for MyHelper {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Validator for MyHelper {}

impl Highlighter for MyHelper {}

impl Completer for MyHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        let start = line[..pos].rfind(' ').map_or(0, |i| i + 1);
        let prefix = &line[start..pos];

        let mut matches = self.completer.autocomplete(prefix);

        matches.sort_by(|a, b| a.display().cmp(b.display()));

        Ok((start, matches))
    }
}

fn main() -> rustyline::Result<()> {
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .bell_style(BellStyle::Audible)
        .auto_add_history(true)
        .build();

    let mut editor = Editor::with_config(config)?;
    editor.set_helper(Some(MyHelper {
        completer: AutoCompleter::new(),
    }));

    let history_file_path = match env::var("HISTFILE") {
        Ok(path) => path,
        Err(_) => "history.txt".to_string(),
    };

    let _ = editor.load_history(&history_file_path);

    loop {
        let readline = editor.readline("$ ");
        let input = match readline {
            Ok(line) => {
                editor.add_history_entry(line.as_str())?;
                line
            }
            Err(ReadlineError::Interrupted) => {
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        };

        let parsed = parse_command_line(input.trim());
        let valid_commands = ["echo", "exit", "type", "pwd", "history"];

        if parsed.is_empty() {
            continue;
        }

        let cmd = &parsed[0];
        let args = &parsed[1..];

        let redirect_op_index = args.iter().position(|s| {
            s == ">" || s == "1>" || s == "2>" || s == ">>" || s == "1>>" || s == "2>>"
        });

        if cmd == "exit" {
            break;
        } else if cmd == "history" {
            if args.len() >= 2 {
                match args[0].as_str() {
                    "-r" => {
                        let file_path = &args[1];
                        match editor.load_history(file_path) {
                            Ok(_) => (),
                            Err(e) => eprintln!("Failed to load history from {}: {}", file_path, e),
                        }
                    }
                    "-w" => {
                        let file_path = &args[1];

                        match editor.save_history(file_path) {
                            Ok(_) => {
                                if let Ok(content) = fs::read_to_string(file_path) {
                                    let cleaned_content = content.replace("#V2\n", "");
                                    let _ = fs::write(file_path, cleaned_content);
                                }
                            }
                            Err(e) => eprintln!("Failed to save history to {}: {}", file_path, e),
                        }
                    }
                    "-a" => {
                        let file_path = &args[1];

                        match editor.append_history(file_path) {
                            Ok(_) => {
                                if let Ok(content) = fs::read_to_string(file_path) {
                                    let cleaned_content = content.replace("#V2\n", "");
                                    let _ = fs::write(file_path, cleaned_content);
                                }
                            }
                            Err(e) => eprintln!("Failed to append history to {}: {}", file_path, e),
                        }
                    }
                    _ => {
                        eprintln!("history: unknown option {}", args[0]);
                    }
                }
            } else if args.len() == 1 {
                if let Ok(n) = args[0].parse::<usize>() {
                    let history = editor.history();
                    let total_len = history.len();
                    let start_idx = total_len.saturating_sub(n);

                    for (i, entry) in history.iter().enumerate().skip(start_idx) {
                        println!("  {} {}", i + 1, entry);
                    }
                } else {
                    eprintln!("history: invalid number {}", args[0]);
                }
            } else {
                let history = editor.history();
                for (i, entry) in history.iter().enumerate() {
                    println!("  {} {}", i + 1, entry);
                }
            }
        } else if cmd == "cd" {
            let target = if args.len() > 0 && args[0] == "~" {
                match env::var("HOME") {
                    Ok(h) => h,
                    Err(_) => {
                        eprintln!("cd: HOME not set");
                        continue;
                    }
                }
            } else {
                args[0].to_string()
            };

            if let Err(_) = env::set_current_dir(&target) {
                eprintln!("cd: {}: No such file or directory", target);
            }
        } else if cmd == "type" {
            if args.is_empty() {
                eprintln!("type: missing argument");
            } else {
                // assuming type command will have one only one argument. e.g. type cat, type ls, type cat ls will not work
                let arg = &args[0];
                if valid_commands.contains(&arg.as_str()) {
                    println!("{} is a shell builtin", arg);
                } else if let Some(executable_path) = find_executable(arg) {
                    println!("{} is {}", arg, executable_path);
                } else {
                    println!("{}: not found", arg);
                }
            }
        } else if cmd == "pwd" {
            // [Bug]: pwd executable missing in test environment
            let path = match env::current_dir() {
                Ok(p) => p,
                Err(e) => {
                    println!("Error: {}", e);
                    return Ok(());
                }
            };
            println!("{}", path.display());
        } else if find_executable(cmd).is_some() {
            // @TODO let Some(redirect_idx) = redirect_op_index: computed twice
            let new_args = if let Some(redirect_idx) = redirect_op_index {
                &args[..redirect_idx]
            } else {
                args
            };

            let output = Command::new(cmd).args(new_args).output();

            match output {
                Ok(output) => {
                    if let Some(idx) = redirect_op_index {
                        // Take the path after '>' as a single token
                        let operator = args.get(idx).expect("operator not found");
                        let path_token = args.get(idx + 1).expect("No path after redirection");

                        // @TODO: Handle error gracefully instead of panicking where parent directory doesn't exist (e.g., mydir/file.md when mydir/ doesn't exist)
                        match operator.as_str() {
                            ">" | "1>" => {
                                fs::write(path_token.as_str(), &output.stdout)
                                    .expect("Failed to write to stdout");
                                eprint!("{}", String::from_utf8_lossy(&output.stderr));
                            }
                            "2>" => {
                                fs::write(path_token.as_str(), &output.stderr)
                                    .expect("Failed to write to stderr");
                                print!("{}", String::from_utf8_lossy(&output.stdout));
                            }
                            ">>" | "1>>" => {
                                use std::fs::OpenOptions;
                                use std::io::Write;
                                let mut file = OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open(path_token.as_str())
                                    .expect("Failed to open file for appending");
                                file.write_all(&output.stdout)
                                    .expect("Failed to append to file");

                                eprint!("{}", String::from_utf8_lossy(&output.stderr));
                            }
                            "2>>" => {
                                use std::fs::OpenOptions;
                                use std::io::Write;
                                let mut file = OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open(path_token.as_str())
                                    .expect("Failed to open file for appending");
                                file.write_all(&output.stderr)
                                    .expect("Failed to append to file");

                                print!("{}", String::from_utf8_lossy(&output.stdout));
                            }
                            _ => eprintln!("Unsupported redirect operator: {}", operator),
                        };
                    } else {
                        eprint!("{}", String::from_utf8_lossy(&output.stderr));
                        print!("{}", String::from_utf8_lossy(&output.stdout));
                        io::stdout().flush().unwrap();
                    }
                }
                Err(e) => println!("{}", e),
            }
        } else if !cmd.is_empty() {
            println!("{}: command not found", cmd);
        }
    }

    match editor.save_history(&history_file_path) {
        Ok(_) => {
            if let Ok(content) = fs::read_to_string(&history_file_path) {
                let cleaned_content = content.replace("#V2\n", "");
                let _ = fs::write(&history_file_path, cleaned_content);
            }
        }
        Err(e) => eprintln!("Failed to save history to {}: {}", &history_file_path, e),
    };

    Ok(())
}

fn find_executable(command: &str) -> Option<String> {
    let path_var = env::var("PATH").ok()?;

    for mut file_path in env::split_paths(&path_var) {
        file_path.push(command);

        if Path::new(&file_path).exists() {
            if let Ok(md) = fs::metadata(&file_path) {
                if (md.permissions().mode() & 0o100) != 0 {
                    return Some(file_path.display().to_string());
                }
            }
        }
    }
    None
}

fn parse_command_line(input: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut current_arg = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == ' ' && !in_single_quote && !in_double_quote {
            if !current_arg.is_empty() {
                arguments.push(current_arg.clone());
                current_arg.clear();
            }
        } else if ch == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if ch == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if ch == '\\' {
            if in_single_quote {
                current_arg.push(ch);
            } else if in_double_quote {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '\\' || next_ch == '"' || next_ch == '$' || next_ch == '`' {
                        chars.next();
                        current_arg.push(next_ch);
                    } else {
                        current_arg.push(ch);
                    }
                } else {
                    current_arg.push(ch);
                }
            } else {
                if let Some(next_ch) = chars.next() {
                    current_arg.push(next_ch);
                }
            }
        } else {
            current_arg.push(ch);
        }
    }

    if !current_arg.is_empty() {
        arguments.push(current_arg);
    }

    arguments
}
