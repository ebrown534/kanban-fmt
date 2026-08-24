mod error;
mod model;
mod parser;
mod printer;

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (check_only, path) = match parse_args(&args) {
        Ok(v) => v,
        Err(message) => {
            eprintln!("{}", message);
            process::exit(2);
        }
    };

    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read '{}': {}", path, e);
            process::exit(2);
        }
    };

    match parser::parse(&source) {
        Ok(board) => {
            if check_only {
                let card_count: usize = board.columns.iter().map(|c| c.cards.len()).sum();
                println!(
                    "ok: {} column(s), {} card(s)",
                    board.columns.len(),
                    card_count
                );
            } else {
                print!("{}", printer::pretty_print(&board));
            }
        }
        Err(errors) => {
            for (idx, err) in errors.iter().enumerate() {
                if idx > 0 {
                    eprintln!();
                }
                eprint!("{}", err.render(&path, &source));
            }
            if errors.len() > 1 {
                eprintln!("\nerror: aborting due to {} previous errors", errors.len());
            }
            process::exit(1);
        }
    }
}

fn parse_args(args: &[String]) -> Result<(bool, String), String> {
    let mut check_only = false;
    let mut path: Option<String> = None;

    for arg in &args[1..] {
        if arg == "--check" {
            check_only = true;
        } else if arg == "-h" || arg == "--help" {
            return Err(usage());
        } else if path.is_none() {
            path = Some(arg.clone());
        } else {
            return Err(format!("unexpected argument '{}'\n\n{}", arg, usage()));
        }
    }

    match path {
        Some(p) => Ok((check_only, p)),
        None => Err(usage()),
    }
}

fn usage() -> String {
    "usage: kanban-fmt [--check] <file>\n\n  <file>     path to a kanban board export\n  --check    validate only, do not print the formatted board".to_string()
}
