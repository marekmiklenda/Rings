use std::fmt;
use std::fmt::Formatter;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Cursor, Seek, SeekFrom, Write};
use std::iter::Iterator;
use std::result::Result::{Err, Ok};
use std::str::Split;

use byteorder::{ReadBytesExt, WriteBytesExt};

use i_defs::{I_ARGS, I_FUNC, I_KEYS, I_SIZE};
use root::{CompileError, Program, ProgramEnvironment, RuntimeError};

mod i_defs;
mod root;

#[derive(Debug)]
pub enum RingsCompileError {
    LinedRingsCompileError {
        line: String,
        line_number: u16,
        e: CompileError,
    },
    IOError(std::io::Error),
}

impl fmt::Display for RingsCompileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use RingsCompileError::*;

        match self {
            LinedRingsCompileError { line, line_number, e } => write!(f, "{}\nAt {:0>4X}: {}", e, line_number, line),
            IOError(e) => write!(f, "{}", e),
        }
    }
}

fn unwrap_instr(i: u8, reader: &mut BufReader<File>, program: &mut Program) -> Result<(), ()> {
    let mut vec: Vec<u8> = Vec::new();
    vec.push(i);
    for _ in 0..I_SIZE[i as usize] {
        match reader.read_u8() {
            Ok(val) => vec.push(val),
            Err(_) => { return Err(()); }
        }
    }

    program.push(vec);
    Ok(())
}

#[allow(dead_code)]
pub fn compile(source: &str, output: &str) -> Result<(), RingsCompileError> {
    use RingsCompileError::*;
    use CompileError::*;

    match File::open(source) {
        Err(e) => Err(IOError(e)),
        Ok(mut src_file) => {
            let mut instructions: Program = Vec::new();
            let mut labels: Vec<(String, u16)> = Vec::new();

            // First iteration to get label names
            let mut tmp_instr_len: u16 = 0;
            let mut line_number: u16 = 0;
            for _line in io::BufReader::new(&src_file).lines() {
                if let Ok(line) = _line {
                    line_number += 1;
                    let line: String = line.trim().to_owned();
                    if line == "" { continue; }

                    let mut args: Split<char> = line.split(' ');
                    let master = args.next().unwrap().to_owned();

                    if let Some(c) = line.chars().next() {
                        if c == '#' { continue; }
                        if c == ':' {
                            if args.count() != 0 { return Err(LinedRingsCompileError { line, line_number, e: SyntaxError }); }
                            if let Some(..) = labels.iter().position(|x| x.0 == master) { return Err(LinedRingsCompileError { line, line_number, e: LabelAlreadyExists(master) }); }

                            labels.push((master, tmp_instr_len));

                            continue;
                        }
                    }

                    if tmp_instr_len == 0xFFFF { return Err(LinedRingsCompileError { line: String::new(), line_number: tmp_instr_len, e: ProgramTooLong }); }
                    tmp_instr_len += 1;
                }
            }

            if let Err(e) = src_file.seek(SeekFrom::Start(0)) {
                return Err(IOError(e));
            }

            match File::create(output) {
                Err(e) => Err(IOError(e)),
                Ok(out_file) => {
                    let mut writer = BufWriter::new(out_file);
                    line_number = 0;

                    // Compile instructions and assign line numbers to labels
                    for line in io::BufReader::new(src_file).lines() {
                        line_number += 1;
                        if let Ok(line) = line {
                            let line: String = line.trim().to_owned();
                            if line == "" { continue; }

                            if let Some(c) = line.chars().next() {
                                if c == '#' || c == ':' { continue; }
                            }

                            let mut args: Split<char> = line.split(' ');
                            let master = args.next().unwrap().to_owned();
                            let args: Vec<&str> = args.collect();

                            if let Some(index) = I_KEYS.iter().position(|s| s == &master) {
                                let mut i_args: Vec<u8> = vec![index as u8];
                                match I_ARGS[index](&mut i_args, &args, &labels) {
                                    Err(e) => return Err(LinedRingsCompileError { line, line_number, e }),
                                    Ok(()) => {
                                        instructions.push(i_args);
                                        if instructions.len() == 2 {
                                            if let Err(e) = writer.write_u8(instructions[0][0] | instructions[1][0] << 4) { return Err(IOError(e)); }
                                            for i in 0..2 {
                                                for j in 1..instructions[i].len() {
                                                    if let Err(e) = writer.write_u8(instructions[i][j]) { return Err(IOError(e)); }
                                                }
                                            }

                                            instructions.pop();
                                            instructions.pop();
                                        }
                                    }
                                }

                                continue;
                            }

                            return Err(LinedRingsCompileError { line, line_number, e: SyntaxError });
                        }
                    }

                    if instructions.len() == 1 { if let Err(e) = writer.write(&instructions[0]) { return Err(IOError(e)); } }
                    if let Err(e) = writer.flush() { return Err(IOError(e)); }
                    return Ok(());
                }
            }
        }
    }
}

#[allow(dead_code)]
pub fn run(program: &str, stdin: fn() -> Result<u8, std::io::Error>, stdout: fn(u8), stderr: fn(u8)) -> Result<u8, RuntimeError> {
    use RuntimeError::IOError;

    match File::open(program) {
        Err(e) => Err(IOError(e)),
        Ok(file) => {
            let mut env = ProgramEnvironment::new(stdin, stdout, stderr);
            let mut reader = BufReader::new(file);
            let mut program: Program = Vec::new();

            loop {
                match reader.read_u8() {
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::UnexpectedEof { return Err(IOError(e)); }
                        break;
                    }
                    Ok(is) => {
                        let i1 = is & 0b1111;
                        let i2 = is >> 4;

                        if let Err(()) = unwrap_instr(i1, &mut reader, &mut program) { break; }
                        if let Err(()) = unwrap_instr(i2, &mut reader, &mut program) { break; }
                    }
                }
            }

            let p_len = program.len() as u16;
            while env.ip < p_len {
                let mut instr: Cursor<&mut [u8]> = Cursor::new(&mut program[env.ip as usize]);
                if let Err(e) = (I_FUNC[instr.read_u8().unwrap() as usize])(&mut env, instr) {
                    return match e {
                        RuntimeError::Halt(c) => Ok(c),
                        _ => Err(e),
                    };
                }

                if env.correction { env.correction = false } else { env.ip += 1; }
            }

            Ok(0)
        }
    }
}

#[allow(dead_code)]
pub fn print_program(program: &Program) {
    for i in 0..program.len() {
        let line = &program[i];
        print!("0x{:0>4X}: ", i);

        for byte in line.iter() {
            print!("{:0>2X} ", byte);
        }

        print!("\n");
    }
}