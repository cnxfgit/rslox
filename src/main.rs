mod chunk;
mod debug;
mod value;
mod vm;
mod compiler;
mod scanner;
mod object;
mod table;
mod memory;
use std::{env, fs, io::{self, Write}, process};
use vm::{InterpretResult, vm};

fn main() -> io::Result<()> {
    vm::init_vm();

    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        repl()?;
    } else if args.len() == 2 {
        run_file(&args[1])?;
    } else {
        eprintln!("Usage: clox [path]");
        process::exit(64);
    }

    vm::drop_vm();

    Ok(())
}

fn repl() -> io::Result<()>  {
    let mut line = String::new();
    loop {
        print!("> ");
        io::stdout().flush()?;
        let result = io::stdin().read_line(&mut line)?;
        if result == 0 {
            break;
        }

        vm().interpret(line.clone());
        line.clear();
    }

    Ok(())
}

fn run_file(path: &str) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    let result = vm().interpret(source);

    match result {
        InterpretResult::CompileError => process::exit(65),
        InterpretResult::RuntimeError => process::exit(70),
        _ => Ok(())
    }
}
