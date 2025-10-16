use shlex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use std::process::{Command, Stdio, ChildStdout};

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

enum CommandType {
    Builtin,
    External,
}

fn classify_command(cmd: &str) -> CommandType {
    let builtins = ["echo", "type", "pwd"];
    if builtins.contains(&cmd) {
        CommandType::Builtin
    } else {
        CommandType::External
    }
}

fn execute_builtin(cmd: &str, args: &[String]) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();

    match cmd {
        "echo" => {
            let text = args.join(" ");
            output.extend_from_slice(text.as_bytes());
            output.push(b'\n');
        }
        "pwd" => {
            let path = env::current_dir()?;
            output.extend_from_slice(path.display().to_string().as_bytes());
            output.push(b'\n');
        }
        "type" => {
            if args.is_empty() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "type: missing argument"));
            }
            let valid_commands = ["echo", "exit", "type", "pwd", "history"];
            let arg = &args[0];

            let result = if valid_commands.contains(&arg.as_str()) {
                format!("{} is a shell builtin\n", arg)
            } else if let Some(executable_path) = find_executable(arg) {
                format!("{} is {}\n", arg, executable_path)
            } else {
                format!("{}: not found\n", arg)
            };

            output.extend_from_slice(result.as_bytes());
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unknown builtin: {}", cmd)
            ));
        }
    }

    Ok(output)
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

        let parsed_opt = parse_command_line(input.trim());
        let valid_commands = ["echo", "exit", "type", "pwd", "history"];

        let (cmd, args) = match parsed_opt {
            Some((p, a)) => (p, a),
            None => continue,
        };

        let cmd_str = cmd.as_str();
        let redirect_op_index = args.iter().position(|s| {
            s == ">" || s == "1>" || s == "2>" || s == ">>" || s == "1>>" || s == "2>>"
        });

        if args.contains(&"|".to_string()) {
            // reconstruct full command with program + args to split on '|'
            let mut full = Vec::with_capacity(1 + args.len());
            full.push(cmd.clone());
            full.extend(args.clone());
            let joined = full.join(" ");
            let parts: Vec<&str> = joined.split('|').map(str::trim).collect();

            match execute_pipeline(parts) {
                Ok(_) => {
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        } else if cmd_str == "exit" {
            break;
        } else if cmd_str == "history" {
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
                        println!("    {}  {}", i + 1, entry);
                    }
                } else {
                    eprintln!("history: invalid number {}", args[0]);
                }
            } else {
                let history = editor.history();
                for (i, entry) in history.iter().enumerate() {
                    println!("    {}  {}", i + 1, entry);
                }
            }
        } else if cmd_str == "cd" {
            let target = if args.len() > 0 && args[0] == "~" {
                match env::var("HOME") {
                    Ok(h) => h,
                    Err(_) => {
                        eprintln!("cd: HOME not set");
                        continue;
                    }
                }
            } else {
                if args.is_empty() {
                    eprintln!("cd: missing argument");
                    continue;
                }
                args[0].to_string()
            };

            if let Err(_) = env::set_current_dir(&target) {
                eprintln!("cd: {}: No such file or directory", target);
            }
        } else if cmd_str == "type" {
            if args.is_empty() {
                eprintln!("type: missing argument");
            } else {
                let arg = &args[0];
                if valid_commands.contains(&arg.as_str()) {
                    println!("{} is a shell builtin", arg);
                } else if let Some(executable_path) = find_executable(arg) {
                    println!("{} is {}", arg, executable_path);
                } else {
                    println!("{}: not found", arg);
                }
            }
        } else if cmd_str == "pwd" {
            let path = match env::current_dir() {
                Ok(p) => p,
                Err(e) => {
                    println!("Error: {}", e);
                    return Ok(());
                }
            };
            println!("{}", path.display());
        } else if find_executable(cmd_str).is_some() {
            let new_args: &[String] = if let Some(redirect_idx) = redirect_op_index {
                &args[..redirect_idx]
            } else {
                &args[..]
            };

            let output = Command::new(cmd_str).args(new_args).output();

            match output {
                Ok(output) => {
                    if let Some(idx) = redirect_op_index {
                        let operator = args.get(idx).expect("operator not found");
                        let path_token = args.get(idx + 1).expect("No path after redirection");

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
        } else if !cmd_str.is_empty() {
            println!("{}: command not found", cmd_str);
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

fn execute_pipeline(commands: Vec<&str>) -> io::Result<()> {
    if commands.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No commands provided"));
    }

    let mut previous_output: Option<Vec<u8>> = None;
    let mut previous_stdout: Option<ChildStdout> = None;
    let mut child_processes = Vec::new();

    for (i, cmd) in commands.iter().enumerate() {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let program = parts[0];
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        match classify_command(program) {
            CommandType::Builtin => {

                let output = execute_builtin(program, &args)?;

                if i == commands.len() - 1 {
                    // Last command: write to stdout
                    io::stdout().write_all(&output)?;
                    io::stdout().flush()?;
                } else {
                    // Store output for next command
                    previous_output = Some(output);
                }
            }
            CommandType::External => {
                let mut command = Command::new(program);
                command.args(&args);

                // Check what kind of input we have
                let has_builtin_output = previous_output.is_some();
                let has_piped_stdout = previous_stdout.is_some();

                // Set stdin based on what we have
                if has_builtin_output {
                    // Previous command was a builtin, we'll pipe its output
                    command.stdin(Stdio::piped());
                } else if has_piped_stdout {
                    // Previous command was external with piped stdout
                    command.stdin(Stdio::from(previous_stdout.take().unwrap()));
                }

                // Set stdout
                command.stdout(if i == commands.len() - 1 {
                    Stdio::inherit()
                } else {
                    Stdio::piped()
                });

                let mut child = command.spawn()?;

                // If we have buffered output from a builtin, write it to stdin
                if has_builtin_output {
                    let output = previous_output.take().unwrap();
                    if let Some(mut stdin) = child.stdin.take() {
                        stdin.write_all(&output)?;
                        drop(stdin);
                    }
                }

                // Store stdout for next command if not last
                if i < commands.len() - 1 {
                    previous_stdout = child.stdout.take();
                }

                child_processes.push(child);
            }
        }
    }

    // Wait for the last process first
    if !child_processes.is_empty() {
        let mut last = child_processes.pop().unwrap();
        let _ = last.wait()?;
    }

    // Clean up remaining processes
    for mut child in child_processes {
        let _ = child.kill();
        let _ = child.wait();
    }

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

fn parse_command_line(cmd: &str) -> Option<(String, Vec<String>)> {
    let parts = shlex::split(cmd)?;
    if parts.is_empty() {
        return None;
    }
    let program = parts[0].clone();
    let args = parts[1..].to_vec();
    Some((program, args))
}