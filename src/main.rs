mod error;
mod model;
mod parser;
mod printer;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

#[derive(PartialEq)]
enum Mode {
    Print,
    Check,
    Write,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (mode, path) = match parse_args(&args) {
        Ok(v) => v,
        Err(message) => {
            eprintln!("{}", message);
            process::exit(2);
        }
    };

    if path == "-" && mode == Mode::Write {
        eprintln!("error: --write requires a file path, stdin has nowhere to write back to");
        process::exit(2);
    }

    let source = if path == "-" {
        let mut buf = String::new();
        match io::stdin().read_to_string(&mut buf) {
            Ok(_) => buf,
            Err(e) => {
                eprintln!("error: could not read stdin: {}", e);
                process::exit(2);
            }
        }
    } else {
        match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: could not read '{}': {}", path, e);
                process::exit(2);
            }
        }
    };

    let display_name = if path == "-" { "<stdin>" } else { &path };

    match parser::parse(&source) {
        Ok(board) => match mode {
            Mode::Check => {
                let card_count: usize = board.columns.iter().map(|c| c.cards.len()).sum();
                println!(
                    "ok: {} column(s), {} card(s)",
                    board.columns.len(),
                    card_count
                );
            }
            Mode::Print => {
                print!("{}", printer::pretty_print(&board));
            }
            Mode::Write => {
                let formatted = printer::pretty_print(&board);
                if formatted != source {
                    if let Err(e) = fs::write(&path, formatted) {
                        eprintln!("error: could not write '{}': {}", path, e);
                        process::exit(2);
                    }
                }
            }
        },
        Err(errors) => {
            for (idx, err) in errors.iter().enumerate() {
                if idx > 0 {
                    eprintln!();
                }
                eprint!("{}", err.render(display_name, &source));
            }
            if errors.len() > 1 {
                eprintln!("\nerror: aborting due to {} previous errors", errors.len());
            }
            process::exit(1);
        }
    }
}

fn parse_args(args: &[String]) -> Result<(Mode, String), String> {
    let mut mode = Mode::Print;
    let mut path: Option<String> = None;

    for arg in &args[1..] {
        if arg == "--check" || arg == "--write" {
            let requested = if arg == "--check" { Mode::Check } else { Mode::Write };
            if mode != Mode::Print && mode != requested {
                return Err(format!("'--check' and '--write' are mutually exclusive\n\n{}", usage()));
            }
            mode = requested;
        } else if arg == "-h" || arg == "--help" {
            return Err(usage());
        } else if path.is_none() {
            path = Some(arg.clone());
        } else {
            return Err(format!("unexpected argument '{}'\n\n{}", arg, usage()));
        }
    }

    match path {
        Some(p) => Ok((mode, p)),
        None => Err(usage()),
    }
}

fn usage() -> String {
    "usage: kanban-fmt [--check | --write] <file>\n\n  <file>     path to a kanban board export, or '-' to read from stdin\n  --check    validate only, do not print the formatted board\n  --write    format the file in place instead of printing to stdout (not valid with '-')".to_string()
}
