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

    // Wait for user input
    let mut input = String::new();
    let valid_commands = ["echo", "exit", "type", "pwd"];


    loop {
        io::stdin().read_line(&mut input).unwrap();
        let input_trimmed = input.trim();

        // let cmd_parts: Vec<&str> = input.split(" ").collect();
        let mut _parts = input.splitn(2, ' ');
        let _cmd = _parts.next().unwrap_or("").trim();
        let args = _parts.next().unwrap_or("").trim();


        if _cmd == "exit" {
            break;
        } else if _cmd == "echo" {
            println!("{args}");
        } else if _cmd == "type" {

            if valid_commands.contains(&args) {
                println!("{args} is a shell builtin")
            } else if let Some(executable_path) = find_executable(args) {
                println!("{args} is {executable_path}");
            } else {
                println!("{args}: not found");
            }
        } else if _cmd == "pwd" {
            let path = match env::current_dir() {
                Ok(p) => p,
                Err(e) => {
                    println!("Error: {}", e);
                    return
                },
            };
            println!("{}", path.display());
        } else if _cmd == "cd" {

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


        } else if find_executable(_cmd).is_some() {

            let args_list: Vec<&str> = args.split(' ').collect();

            let output = Command::new(_cmd)
                                    .args(&args_list)
                                    .output()
                                    .expect("failed to execute process");
            // println!("\n{:?}\n", args_list);

            print!("{}", String::from_utf8_lossy(&output.stdout));
            io::stdout().flush().unwrap();
        } else if input_trimmed != "" {
            println!("{input_trimmed}: command not found");
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