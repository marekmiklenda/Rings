use std::env;
use std::io::Write;
use std::iter::Iterator;

use byteorder::{ReadBytesExt, WriteBytesExt};

mod rings;

fn help() {
    eprintln!("usage:\ncompile [-h] SOURCE OUTPUT\nrun [-h] PROGRAM\n\noptional arguments:\n-h, --help       show this help message and exit\n\npositional arguments:\nSOURCE           path to a .hrn file to compile\nOUTPUT           path to put the resulting .rn file\nPROGRAM          path to a .rn file to execute");
}

fn panic_help() -> ! {
    help();
    std::process::exit(64);
}

fn fn_stderr(e: u8) { std::io::stderr().write_u8(e); }

fn fn_stdout(o: u8) { std::io::stdout().write_u8(o); }

fn fn_stdin() -> Result<u8, std::io::Error> {
    return std::io::stdin().read_u8();
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 { panic_help() }

    match args[1].as_str() {
        "-h" | "--help" => {
            help();
            return;
        }
        "compile" => {
            if args.len() != 4 { panic_help() }

            match rings::compile(&args[2], &args[3]) {
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                Ok(()) => {
                    println!("Done compiling");
                    return;
                }
            }
        }
        "run" => {
            if args.len() != 3 { panic_help() }

            match rings::run(&args[2], fn_stdin, fn_stdout, fn_stderr) {
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                Ok(v) => std::process::exit(v as i32)
            }
        }
        _ => panic_help()
    }
}
