use std::{collections::HashMap, io::stdin};

use byteorder::ReadBytesExt;
use clap::Parser;
use rings::{debug, execute, precompile, Ring, RingsError, RingsErrorKind};

#[derive(Parser, Debug)]
#[clap(author = "Marek Miklenda")]
#[clap(version)]
#[clap(about="Rings interpreter", long_about = None)]
struct Args {
    /// File to run
    file: String,

    /// Verbose mode
    #[clap(short, long, action)]
    verbose: bool,

    /// Lines to debug, can specify multiple times
    #[clap(short, long, value_parser)]
    breakpoints: Option<Vec<usize>>,

    /// Disable generation of debug symbols
    #[clap(short, long, action)]
    no_debug: bool,

    /// Manually input stdin values, can specify multiple times
    #[clap(short, long, value_parser)]
    stdin: Option<Vec<u8>>,
}

fn print_error(error: RingsError, debug_symbols: HashMap<usize, usize>) {
    let line = *debug_symbols.get(&error.0).unwrap_or(&error.0);

    use RingsErrorKind::*;

    match error.1 {
        IOError(e) => println!("{}", e),
        SyntaxError(s) => println!("Syntax error on line {}: {}", line, s),
        TooLarge => println!("Attempting to address outside address range (0xFFFF)"),
        UndeclaredLabel(lbl) => println!("Use of undeclared label {}", lbl),
        NonexistentRing(x) => println!(
            "Attempting to manipulate nonexistent ring {} on line {}",
            x, line
        ),
        TooManyRings => println!("Cannot declare any more rings at line {}", line),
        InvalidValue(v) => println!("Attempting to use invalid value {} at line {}", v, line),
        _ => {}
    }
}

fn main() {
    let args = Args::parse();

    let (bytecode, debug_symbols) = match precompile(&args.file, !args.no_debug, args.verbose) {
        Ok(prog) => prog,
        Err(e) => {
            print_error(e, HashMap::new());
            return;
        }
    };

    let mut term = stdin();

    let stdin_override = args.stdin.unwrap_or_default();
    let mut stdin_override = stdin_override.iter();
    let stdin = || -> u8 {
        stdin_override
            .next()
            .map_or_else(|| term.read_u8().unwrap_or(0), |x| *x)
    };

    let stdout = |x: u8| println!("{:<2X}", x);
    let stderr = |x: u8| eprintln!("{:<2X}", x);

    match match args.breakpoints {
        Some(dbg) => {
            let term2 = std::io::stdin();
            let debug_callback = |offset: usize, state: &[Ring]| {
                if let Some(line) = debug_symbols.get(&offset) {
                    if !dbg.contains(line) {
                        return;
                    }

                    println!("Breakpoint on line {}", line);
                    for (i, ring) in state.iter().enumerate() {
                        println!("{:0>2X}: {}", i, ring);
                    }

                    let _ = term2.read_line(&mut String::new());
                }
            };

            debug(bytecode, (stdin, stdout, stderr), debug_callback)
        }
        None => execute(bytecode, (stdin, stdout, stderr)),
    } {
        Ok(exit) => println!("\nProcess finished with exit code {}", exit),
        Err(e) => print_error(e, debug_symbols),
    };
}
