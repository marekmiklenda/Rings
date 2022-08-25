use std::fmt;
use std::fmt::Formatter;
use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct Instruction(pub u8, pub Vec<u8>);

pub type Program = Vec<Vec<u8>>;

#[derive(Debug)]
pub struct Ring {
    array: Vec<u8>,
    offset: u8,
}

#[derive(Debug)]
pub struct ProgramEnvironment {
    rings: Vec<Ring>,
    pub correction: bool,
    pub ip: u16,
    pub stdin: fn() -> Result<u8, std::io::Error>,
    pub stdout: fn(u8) -> Result<(), std::io::Error>,
    pub stderr: fn(u8) -> Result<(), std::io::Error>,
}

#[derive(Debug)]
pub enum CompileError {
    SyntaxError,
    TypeMismatch { expected: String, got: String },
    InvalidValue(usize),
    LabelNotFound(String),
    LabelAlreadyExists(String),
    ProgramTooLong,
}

#[derive(Debug)]
pub enum RuntimeError {
    IOError(std::io::Error),
    IndexOutOfBounds { max: usize, got: usize },
    InvalidValue(isize),
    RingLimit,
    DivideByZero,
    Halt(u8),
    StdioReadError(std::io::Error),
    StdioWriteError(std::io::Error),
}

impl Ring {
    pub fn len(&self) -> u8 { self.array.len() as u8 }

    pub fn add_offset(&mut self, offset: u8) {
        self.offset = ((self.offset as u16 + offset as u16) % self.len() as u16) as u8;
    }
}

impl Index<u8> for Ring {
    type Output = u8;
    fn index(&self, i: u8) -> &u8 {
        &self.array[((i as u16 + self.len() as u16 - self.offset as u16) % self.len() as u16) as usize]
    }
}

impl IndexMut<u8> for Ring {
    fn index_mut(&mut self, i: u8) -> &mut u8 {
        let len = self.len() as u16;
        &mut self.array[((i as u16 + len - self.offset as u16) % len) as usize]
    }
}

impl fmt::Display for Ring {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "(+{:0>2X})", self.offset)?;

        for i in 0..self.len()
        {
            write!(f, "[{:0>2X}]", self[i])?;
        }

        write!(f, "")
    }
}

impl ProgramEnvironment {
    pub fn new(stdin: fn() -> Result<u8, std::io::Error>, stdout: fn(u8) -> Result<(), std::io::Error>, stderr: fn(u8) -> Result<(), std::io::Error>) -> ProgramEnvironment {
        ProgramEnvironment {
            rings: Vec::new(),
            correction: false,
            ip: 0,
            stdin,
            stdout,
            stderr,
        }
    }

    pub fn len(&self) -> u16 { self.rings.len() as u16 }

    pub fn mkring(&mut self, size: u8) { self.rings.push(Ring { array: vec![0; size as usize], offset: 0 }) }

    pub fn mv_ip(&mut self, new_pos: u16) {
        self.ip = new_pos;
        self.correction = true;
    }
}

impl Index<u8> for ProgramEnvironment {
    type Output = Ring;
    fn index(&self, i: u8) -> &Ring { &self.rings[i as usize] }
}

impl IndexMut<u8> for ProgramEnvironment {
    fn index_mut(&mut self, i: u8) -> &mut Ring { &mut self.rings[i as usize] }
}

impl fmt::Display for ProgramEnvironment {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for i in 0..=(self.len() - 1) as u8
        {
            writeln!(f, "0x{:0>2X}: {}", i, self[i])?;
        }

        write!(f, "")
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use CompileError::*;

        match self {
            SyntaxError => write!(f, "Syntax error"),
            TypeMismatch { expected, got } => write!(f, "Type mismatch: expected {}, got {}", expected, got),
            InvalidValue(val) => write!(f, "Invalid value: {}", val),
            LabelNotFound(lbl) => write!(f, "Label not found: {}", lbl),
            LabelAlreadyExists(lbl) => write!(f, "Label already exists: {}", lbl),
            ProgramTooLong => write!(f, "Program is too long! Max number of instructions: 65535 (0xFFFF)"),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use RuntimeError::*;

        match self {
            IndexOutOfBounds { max, got } => write!(f, "Index out of bounds: {}, with max {}", got, max),
            InvalidValue(val) => write!(f, "Invalid value: {}", val),
            RingLimit => write!(f, "Ring limit reached: 255 rings have already been created"),
            DivideByZero => write!(f, "Attempt to divide by zero"),
            Halt(c) => write!(f, "Process finished with exit code {}", c),
            StdioReadError(e) => write!(f, "Error reading from stdin: {}", e),
            IOError(e) => write!(f, "Error reading from file: {}", e),
            StdioWriteError(e) => write!(f, "Error writing to stdout/stderr: {}", e),
        }
    }
}