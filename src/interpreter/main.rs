use std::{env, fs, io};

mod scanner;
use scanner::Scanner;
mod token;
use token::Token;
mod util;
use util::{had_error_get, had_error_set};
mod object;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        println!("Usage: rslox [script]");
        std::process::exit(64);
    } else if args.len() == 2 {
        run_file(&args[1])?;
    } else {
        run_prompt()?;
    }

    Ok(())
}

fn run_file(path: &String) -> io::Result<()> {
    let string = fs::read_to_string(path)?;
    run(string)?;

    if had_error_get() {
        std::process::exit(65);
    }

    Ok(())
}

fn run(source: String) -> io::Result<()> {
    let mut scanner = Scanner::new(source);
    let tokens: &Vec<Token> = scanner.scan_tokens();

    for token in tokens {
        println!("{}", token);
    }

    Ok(())
}

fn run_prompt() -> io::Result<()> {
    let mut input = String::new();

    loop {
        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                if n == 1 {
                    return Ok(());
                }
                run(input.clone())?
            }
            Err(e) => return io::Result::Err(e),
        }
        input.clear();
        had_error_set(false);
    }
}
