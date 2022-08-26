use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    num::ParseIntError,
    time::Instant,
    vec,
};

use i_defs::Instruction;

pub mod i_defs;

pub type Program = (Vec<u8>, HashMap<usize, usize>);
pub type IO<I, O, E> = (I, O, E);

use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

#[derive(Debug)]
pub struct Ring {
    offset: u8,
    strip: Vec<u8>,
}

#[allow(clippy::len_without_is_empty)]
impl Ring {
    pub fn new(size: u8) -> Self {
        Ring {
            offset: 0,
            strip: vec![0u8; size as usize],
        }
    }

    pub fn len(&self) -> u8 {
        self.strip.len() as u8
    }

    pub fn add_offset(&mut self, offset: u8) {
        self.offset = ((self.offset as u16 + offset as u16) % self.len() as u16) as u8;
    }
}

impl Display for Ring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(+{:0>2X})", self.offset)?;

        for val in self.strip.iter() {
            write!(f, "[{:0>2X}]", val)?;
        }

        Ok(())
    }
}

impl Index<u8> for Ring {
    type Output = u8;
    fn index(&self, i: u8) -> &u8 {
        &self.strip[(i as usize + self.len() as usize - self.offset as usize) % self.len() as usize]
    }
}

impl IndexMut<u8> for Ring {
    fn index_mut(&mut self, i: u8) -> &mut u8 {
        let len = self.len() as usize;
        &mut self.strip[(i as usize + len - self.offset as usize) % len]
    }
}

#[derive(Debug)]
pub enum RingsErrorKind {
    IOError(std::io::Error),
    SyntaxError(String),
    TooLarge,
    UndeclaredLabel(String),
    InvalidValue(i32),
    NonexistentRing(u8),
    TooManyRings,
    Halt(u8),
    Jump(usize),
}

#[derive(Debug)]
pub struct RingsError(pub usize, pub RingsErrorKind);

impl From<std::io::Error> for RingsError {
    fn from(e: std::io::Error) -> Self {
        RingsError(0, RingsErrorKind::IOError(e))
    }
}

pub fn precompile(
    path: &str,
    gen_debug_symbols: bool,
    verbose: bool,
) -> Result<Program, RingsError> {
    fn sn_error(line_number: usize, line: &str) -> Result<Program, RingsError> {
        Err(RingsError(
            line_number + 1,
            RingsErrorKind::SyntaxError(line.to_owned()),
        ))
    }

    fn parse_u8(itm: &str) -> Result<u8, ParseIntError> {
        if itm == "0" {
            return Ok(0);
        }

        if let Some(val) = itm.strip_prefix("0b") {
            return u8::from_str_radix(val, 2);
        }

        if let Some(val) = itm.strip_prefix("0B") {
            return u8::from_str_radix(val, 2);
        }

        if let Some(val) = itm.strip_prefix("0x") {
            return u8::from_str_radix(val, 16);
        }

        if let Some(val) = itm.strip_prefix('0') {
            return u8::from_str_radix(val, 8);
        }

        itm.parse()
    }

    let start = Instant::now();

    let log = |m: &str| {
        if verbose {
            println!("{}", m);
        }
    };

    let file = File::open(path)?;

    let mut label_declarations: HashMap<String, u16> = HashMap::new();
    let mut label_accesses: Vec<String> = vec![];
    let mut label_substitutions: HashMap<u16, String> = HashMap::new();

    let mut bytecode: Vec<u8> = vec![];
    let mut debug_symbols: HashMap<usize, usize> = HashMap::new();

    for (line_number, line) in BufReader::new(&file).lines().flatten().enumerate() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        log(line);

        let mut line_destr = line.split(' ');

        if line.starts_with(':') {
            if line_destr.count() != 1 {
                return sn_error(line_number, &format!("Invalid label name: {}", line));
            }

            if label_declarations
                .iter()
                .filter(|(l, _)| l.as_str() == line)
                .count()
                != 0
            {
                return sn_error(line_number, &format!("Label already exists: {}", line));
            }

            if bytecode.len() >= 0x1000 {
                return Err(RingsError(line_number, RingsErrorKind::TooLarge));
            }

            label_declarations.insert(line.to_owned(), (bytecode.len()) as u16);

            continue;
        }

        if gen_debug_symbols {
            debug_symbols.insert(bytecode.len() as usize, line_number + 1);
        }

        let instr = match line_destr.next() {
            Some(val) => val,
            None => return sn_error(line_number, line),
        };
        let instr = match Instruction::try_from(instr) {
            Ok(val) => val,
            Err(_) => return sn_error(line_number, line),
        };

        bytecode.push((&instr).into());

        let mut a_count = instr.get_arg_size() as usize;
        let a_map = instr.get_arg_map();
        for (i, itm) in line_destr.enumerate() {
            if a_count == 0 {
                return sn_error(line_number, line);
            }

            if a_map >> i & 1 == 1 {
                // Label
                a_count -= 2;

                if !itm.starts_with(':') {
                    return sn_error(line_number, line);
                }

                let index = match label_accesses.iter().position(|x| x == itm) {
                    Some(pos) => pos as u16,
                    None => {
                        label_accesses.push(itm.to_owned());
                        (label_accesses.len() - 1) as u16
                    }
                };

                label_substitutions.insert(bytecode.len() as u16, itm.to_owned());

                bytecode.push((index >> 8) as u8);
                bytecode.push((index & 0xFF) as u8);
            } else {
                // Byte
                a_count -= 1;

                match parse_u8(itm) {
                    Ok(val) => bytecode.push(val),
                    Err(_) => return sn_error(line_number, line),
                }
            }
        }

        if a_count != 0 {
            return sn_error(line_number, line);
        }
    }

    log("");

    for (offset, key) in label_substitutions.iter() {
        let tgt_offset = match label_declarations.get(key) {
            Some(val) => val,
            None => {
                return Err(RingsError(
                    0,
                    RingsErrorKind::UndeclaredLabel(key.to_owned()),
                ))
            }
        };

        log(&format!("Substituting {}", key));

        let offset = *offset as usize;
        bytecode[offset] = (tgt_offset >> 8) as u8;
        bytecode[offset + 1] = (tgt_offset & 0xFF) as u8;
    }

    if verbose {
        println!();
        print_program(&bytecode);
    }

    log(&format!(
        "\nCompilation done in {}µs",
        start.elapsed().as_micros()
    ));
    Ok((bytecode, debug_symbols))
}

pub fn execute<I, O, E>(bytecode: Vec<u8>, io: IO<I, O, E>) -> Result<u8, RingsError>
where
    I: FnMut() -> u8,
    O: Fn(u8),
    E: Fn(u8),
{
    debug(bytecode, io, |_, _| {})
}

pub fn debug<I, O, E, C>(
    bytecode: Vec<u8>,
    mut io: IO<I, O, E>,
    debug_callback: C,
) -> Result<u8, RingsError>
where
    I: FnMut() -> u8,
    O: Fn(u8),
    E: Fn(u8),
    C: Fn(usize, &[Ring]),
{
    let mut ip: usize = 0;
    let mut rings: Vec<Ring> = vec![];

    while ip < bytecode.len() {
        debug_callback(ip, &rings);

        let instr = Instruction::try_from(bytecode[ip]).unwrap();
        let arg_size = instr.get_arg_size() as usize;
        let args = &bytecode[ip + 1..ip + arg_size + 1];

        if let Err(e) = instr.run(args, &mut rings, &mut io) {
            if let RingsErrorKind::Halt(v) = e {
                return Ok(v);
            }

            if let RingsErrorKind::Jump(v) = e {
                ip = v;
                continue;
            }

            return Err(RingsError(ip, e));
        }

        ip = ip + arg_size + 1;
    }

    Ok(0)
}

pub fn print_program(prog: &[u8]) {
    for (i, byte) in prog.iter().enumerate() {
        print!("{:0>2X} ", byte);

        if i % 16 == 15 {
            println!();
        }
    }

    println!();
}