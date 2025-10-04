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

        let mut _parts = input.splitn(2, ' ');
        let cmd = _parts.next().unwrap_or("").trim();
        let args = _parts.next().unwrap_or("").trim();
        let parsed_args = parse_command_line(&args);
        let valid_commands = ["echo", "exit", "type", "pwd"];

        if cmd == "" {
        } else if cmd == "exit" {
            break;
        } else if cmd == "cd" {

            let target = if args == "~" {
                match env::var("HOME") {
                    Ok(h) => h,
                    Err(_) => {
                        eprintln!("cd: HOME not set");
                        continue;
                    }
                }
            } else {
                args.to_string()
            };

            if let Err(_) = env::set_current_dir(&target) {
                eprintln!("cd: {}: No such file or directory", target);
            }
        } else if cmd == "type" {

            if valid_commands.contains(&args) {
                println!("{args} is a shell builtin")
            } else if let Some(executable_path) = find_executable(args) {
                println!("{args} is {executable_path}");
            } else {
                println!("{args}: not found");
            }
        } else if find_executable(cmd).is_some() {

            let output = Command::new(cmd)
                                    .args(&parsed_args)
                                    .output();

            match output {
                Ok(output) => {
                    print!("{}", String::from_utf8_lossy(&output.stdout));
                    io::stdout().flush().unwrap();
                },
                Err(e) => eprintln!("{e}")
            }
        } else if cmd != "" {
            println!("{cmd}: command not found");
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
    let mut escape_next = false;

    for ch in input.chars() {

        if escape_next {
            current_arg.push(ch);
            escape_next = !escape_next;
        } else if ch == ' ' && !in_single_quote && !in_double_quote {
            if !current_arg.is_empty() {
                arguments.push(current_arg.clone());
                current_arg.clear();
            }
        } else if ch == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if ch == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if ch == '\\' && !in_single_quote && !in_double_quote {
            escape_next = true;
        } else {
            current_arg.push(ch);
        }
    }

    if !current_arg.is_empty() {
        arguments.push(current_arg);
    }

    arguments
}