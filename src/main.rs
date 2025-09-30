#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    // Wait for user input
    let mut input = String::new();

    while input != "" {
        io::stdin().read_line(&mut input).unwrap();
        print!("$ {input}: command not found");
    }



}
