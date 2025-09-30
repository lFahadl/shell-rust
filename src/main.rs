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
        let input = input.trim();

        if input == "" {
            println!("$ ");
        } else {
            println!("{input}: command not found");
        }

    }



}
