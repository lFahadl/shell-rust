#[allow(unused_imports)]
use std::io::{self, Write};
use std::env;
use std::fs;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;




fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    let mut input = String::new();


    loop {
        io::stdin().read_line(&mut input).unwrap();

        let parsed = parse_command_line(input.trim());
        let valid_commands = ["echo", "exit", "type", "pwd"];

        if parsed.is_empty() {
            print!("$ ");
            io::stdout().flush().unwrap();
            input.clear();
            continue;
        }

        let cmd = &parsed[0];
        let args = &parsed[1..];

        if cmd == "exit" {
            break;
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
                let arg = &args[0];
                if valid_commands.contains(&arg.as_str()) {
                    println!("{} is a shell builtin", arg);
                } else if let Some(executable_path) = find_executable(arg) {
                    println!("{} is {}", arg, executable_path);
                } else {
                    println!("{}: not found", arg);
                }
            }
        } else if find_executable(cmd).is_some() {
            let output = Command::new(cmd)
                                    .args(args)
                                    .output();

            match output {
                Ok(output) => {
                    print!("{}", String::from_utf8_lossy(&output.stdout));
                    io::stdout().flush().unwrap();
                },
                Err(e) => eprintln!("{}", e)
            }
        } else if !cmd.is_empty() {
            println!("{}: command not found", cmd);
        }

        print!("$ ");
        io::stdout().flush().unwrap();
        input.clear();
    }
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