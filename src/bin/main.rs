#![feature(try_trait_v2)]
use std::{fs::File, path::PathBuf};

use clap::Parser;
use rings::{
    build::ProgramAssembler,
    error::{MaybeLocalizedRingsResult, RingsError},
    io::SystemStdio,
    vm::RingsVM,
};

#[derive(Parser, Debug)]
#[clap(author = "Marek Miklenda")]
#[clap(version)]
#[clap(about="Rings interpreter", long_about = None)]
struct Args {
    /// File to run
    file: PathBuf,

    /// Disable debugging. No trace will be provided on error.
    #[clap(long, action)]
    no_debug: bool,
}

fn main_wrapped() -> MaybeLocalizedRingsResult<u8> {
    let args = Args::parse();

    let program_file = File::open(args.file).map_err(RingsError::from)?;
    let program = ProgramAssembler::assemble(program_file, !args.no_debug)?.unwrap();

    RingsVM::execute(&program, &mut SystemStdio)
}

fn main() {
    if let Some(e) = main_wrapped().into_err() {
        eprintln!("{}", e);
    }
}
