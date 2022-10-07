use std::{
    collections::HashMap,
    io::{stderr, stdin, stdout},
};

use byteorder::{ReadBytesExt, WriteBytesExt};
use clap::Parser;
use rings::{debug, execute, precompile_file, Ring};

#[derive(Parser, Debug)]
#[clap(author = "Marek Miklenda")]
#[clap(version)]
#[clap(about="Rings interpreter", long_about = None)]
struct Args {
    /// File to run
    file: String,

    /// Verbose mode; stdout/stderr are formatted and printed
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

fn main() {
    let args = Args::parse();

    let (bytecode, debug_symbols) = match precompile_file(&args.file, !args.no_debug, args.verbose)
    {
        Ok(prog) => prog,
        Err(e) => {
            eprintln!("{}", e.format_error(HashMap::new()));
            return;
        }
    };

    let mut stdin = stdin();
    let mut stdout = stdout();
    let mut stderr = stderr();

    let stdin_override = args.stdin.unwrap_or_default();
    let mut stdin_override = stdin_override.iter();

    let stdin_f = || -> u8 {
        stdin_override
            .next()
            .map_or_else(|| stdin.read_u8().unwrap_or(0), |x| *x)
    };
    let stdout_f = |x: u8| {
        if args.verbose {
            println!("{:<2X}", x);
        } else {
            stdout.write_u8(x).ok();
        }
    };
    let stderr_f = |x: u8| {
        if args.verbose {
            eprintln!("{:<2X}", x);
        } else {
            stderr.write_u8(x).ok();
        }
    };

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

            debug(bytecode, (stdin_f, stdout_f, stderr_f), debug_callback)
        }
        None => execute(bytecode, (stdin_f, stdout_f, stderr_f)),
    } {
        Ok(exit) => println!("\nProcess finished with exit code {}", exit),
        Err(e) => eprintln!("{}", e.format_error(debug_symbols)),
    };
}
