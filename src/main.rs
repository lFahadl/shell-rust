#[allow(unused_imports)]
use std::io::{self, Write};
use std::env;
use std::fs;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;



fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    // Wait for user input
    let mut input = String::new();
    let valid_commands = ["echo", "exit", "type"];


    loop {
        let mut flag = false;
        io::stdin().read_line(&mut input).unwrap();
        let input_trimmed = input.trim();

        // let cmd_parts: Vec<&str> = input.split(" ").collect();
        let mut _parts = input.splitn(2, ' ');
        let _cmd = _parts.next().unwrap_or("");
        let _args = _parts.next().unwrap_or("").trim();


        if _cmd == "exit" {
            break;
        } else if _cmd == "echo" {
            println!("{_args}");
        } else if _cmd == "type" {

            if valid_commands.contains(&_args) {
                flag = true;
                println!("{_args} is a shell builtin")
            } else if let Ok(path_var) = env::var("PATH") {
                for mut file_path in env::split_paths(&path_var) {

                    file_path.push(_args);

                    if Path::new(&file_path).exists() {

                        match fs::metadata(&file_path) {
                            Ok(md) => {
                                let permissions = md.permissions();
                                let mode = permissions.mode();
                                let owner_execute = (mode & 0o100) != 0;

                                if owner_execute {
                                    println!("{_args} is {}", file_path.display());
                                    flag = true;
                                    break;
                                }
                            }
                            Err(_) => continue,  // file doesn't exist, try next path
                        }
                    }
                }
            }

            if !flag {
                println!("{_args}: not found");
            }
        } else if input_trimmed != "" {
            println!("{input_trimmed}: command not found");
        }


        print!("$ ");
        io::stdout().flush().unwrap();
        input.clear();
    }
}
