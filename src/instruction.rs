use crate::{io::RingsIo, vm::{Ring, RingId, RingsVM, RuntimeResult}};

pub type Label = usize;
pub type Literal = u8;

type InstructionResult<T> = Result<T, InstructionError>;
#[derive(Debug)]
pub enum InstructionError {
    InvalidInstructionPrimitive((char, char, char)),
    ZeroRingLength,
}

impl std::fmt::Display for InstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInstructionPrimitive((a, b, c)) => {
                write!(f, "Invalid instruction primitive: {}{}{}", a, b, c)
            }
            Self::ZeroRingLength => write!(f, "Cannot create a ring with zero length"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InstructionPrimitive {
    MKR,
    PUT,
    ROT,
    SWP,
    INP,
    OUT,
    ERR,
    ADD,
    SUB,
    MUL,
    DIV,
    JMP,
    JEQ,
    JGT,
    JLT,
    HLT,
}

impl InstructionPrimitive {
    pub fn get_num_args(&self) -> u8 {
        match self {
            Self::MKR => 1,
            Self::PUT => 2,
            Self::ROT => 2,
            Self::SWP => 2,
            Self::INP => 1,
            Self::OUT => 1,
            Self::ERR => 1,
            Self::ADD => 3,
            Self::SUB => 3,
            Self::MUL => 3,
            Self::DIV => 3,
            Self::JMP => 1,
            Self::JEQ => 3,
            Self::JGT => 3,
            Self::JLT => 3,
            Self::HLT => 1,
        }
    }
}

impl TryFrom<(char, char, char)> for InstructionPrimitive {
    type Error = InstructionError;
    fn try_from(value: (char, char, char)) -> Result<Self, Self::Error> {
        match (
            value.0.to_ascii_uppercase(),
            value.1.to_ascii_uppercase(),
            value.2.to_ascii_uppercase(),
        ) {
            ('M', 'K', 'R') => Ok(Self::MKR),
            ('P', 'U', 'T') => Ok(Self::PUT),
            ('R', 'O', 'T') => Ok(Self::ROT),
            ('S', 'W', 'P') => Ok(Self::SWP),
            ('I', 'N', 'P') => Ok(Self::INP),
            ('O', 'U', 'T') => Ok(Self::OUT),
            ('E', 'R', 'R') => Ok(Self::ERR),
            ('A', 'D', 'D') => Ok(Self::ADD),
            ('S', 'U', 'B') => Ok(Self::SUB),
            ('M', 'U', 'L') => Ok(Self::MUL),
            ('D', 'I', 'V') => Ok(Self::DIV),
            ('J', 'M', 'P') => Ok(Self::JMP),
            ('J', 'E', 'Q') => Ok(Self::JEQ),
            ('J', 'G', 'T') => Ok(Self::JGT),
            ('J', 'L', 'T') => Ok(Self::JLT),
            ('H', 'L', 'T') => Ok(Self::HLT),
            _ => Err(InstructionError::InvalidInstructionPrimitive(value)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    MKR(Literal),
    PUT(RingId, Literal),
    ROT(RingId, Literal),
    SWP(RingId, RingId),
    INP(RingId),
    OUT(RingId),
    ERR(RingId),
    ADD(RingId, RingId, RingId),
    SUB(RingId, RingId, RingId),
    MUL(RingId, RingId, RingId),
    DIV(RingId, RingId, RingId),
    JMP(Label),
    JEQ(RingId, RingId, Label),
    JGT(RingId, RingId, Label),
    JLT(RingId, RingId, Label),
    HLT(Literal),
}

impl Instruction {
    pub fn validate(&self) -> InstructionResult<()> {
        match self {
            Self::MKR(0) => Err(InstructionError::ZeroRingLength),
            _ => Ok(()),
        }
    }

    pub fn execute<I>(&self, vm: &mut RingsVM, io: &mut I) -> RuntimeResult<()>
    where
        I: RingsIo,
    {
        macro_rules! arith {
            ($a:expr, $b:expr, $c:expr, $fun:ident) => {{
                let val = (*vm.get_ring(*$a)?.current()).$fun(*vm.get_ring(*$b)?.current());
                *vm.get_ring(*$c)?.current_mut() = val;
            }};
        }

        macro_rules! jumpif {
            ($tgt:expr) => {
                vm.pc = *$tgt
            };

            ($tgt:expr, $a:ident $cmp:tt $b:ident) => {{
                if *vm.get_ring(*$a)?.current() $cmp *vm.get_ring(*$b)?.current() {
                    jumpif!($tgt)
                }
            }};
        }

        match self {
            Self::MKR(capacity) => vm.rings.push(Ring::new(*capacity)?),
            Self::PUT(ring, val) => *vm.get_ring(*ring)?.current_mut() = *val,
            Self::ROT(ring, by) => vm.get_ring(*ring)?.rotate(*by),
            Self::SWP(a, b) => {
                let val_a = *vm.get_ring(*a)?.current();
                let val_b = {
                    let b = vm.get_ring(*b)?.current_mut();
                    let val_b = *b;
                    *b = val_a;
                    val_b
                };

                *(vm.get_ring(*a)?.current_mut()) = val_b;
            }
            Self::INP(ring) => *vm.get_ring(*ring)?.current_mut() = io.inp(vm),
            Self::OUT(ring) => io.out(*vm.get_ring(*ring)?.current(), vm),
            Self::ERR(ring) => io.err(*vm.get_ring(*ring)?.current(), vm),
            Self::ADD(a, b, c) => arith!(a, b, c, wrapping_add),
            Self::SUB(a, b, c) => arith!(a, b, c, wrapping_sub),
            Self::MUL(a, b, c) => arith!(a, b, c, wrapping_mul),
            Self::DIV(a, b, c) => arith!(a, b, c, wrapping_div),
            Self::JMP(tgt) => jumpif!(tgt),
            Self::JEQ(a, b, tgt) => jumpif!(tgt, a == b),
            Self::JGT(a, b, tgt) => jumpif!(tgt, a > b),
            Self::JLT(a, b, tgt) => jumpif!(tgt, a < b),
            Self::HLT(code) => vm.exit_code = Some(*code),
        }

        Ok(())
    }
}
