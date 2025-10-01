#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    // Wait for user input
    let mut input = String::new();
    let valid_commands = ["echo", "exit", "type"];



    loop {
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
                println!("{_args} is a shell builtin")
            } else {
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
