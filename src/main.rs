#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    // Wait for user input
    let mut input = String::new();
    // let flag: bool = true;

    loop {
        io::stdin().read_line(&mut input).unwrap();
        let input_trimmed = input.trim();

        let cmd_parts: Vec<&str> = input.split(" ").collect();

        let _cmd = cmd_parts[0];
        // let _args = cmd_parts[1];



        if _cmd == "exit" {
            break;
        } else if input_trimmed != "" {
            println!("{input_trimmed}: command not found");
        }

        print!("$ ");
        io::stdout().flush().unwrap();
        input.clear();

    }



}
