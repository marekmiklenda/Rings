use crate::{Ring, RingsErrorKind, IO};

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum Instruction {
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

impl Instruction {
    pub fn get_arg_size(&self) -> u8 {
        use Instruction::*;

        match self {
            MKR => 1,
            PUT => 2,
            ROT => 2,
            SWP => 2,
            INP => 1,
            OUT => 1,
            ERR => 1,
            ADD => 3,
            SUB => 3,
            MUL => 3,
            DIV => 3,
            JMP => 2,
            JEQ => 4,
            JGT => 4,
            JLT => 4,
            HLT => 1,
        }
    }

    pub fn get_arg_map(&self) -> u8 {
        use Instruction::*;

        // 0 corresponds to a u8, 1 is a label definition
        match self {
            MKR => 0b00000000,
            PUT => 0b00000000,
            ROT => 0b00000000,
            SWP => 0b00000000,
            INP => 0b00000000,
            OUT => 0b00000000,
            ERR => 0b00000000,
            ADD => 0b00000000,
            SUB => 0b00000000,
            MUL => 0b00000000,
            DIV => 0b00000000,
            JMP => 0b00000001,
            JEQ => 0b00000100,
            JGT => 0b00000100,
            JLT => 0b00000100,
            HLT => 0b00000000,
        }
    }

    pub fn run<I, O, E>(
        &self,
        args: &[u8],
        rings: &mut Vec<Ring>,
        io: &mut IO<I, O, E>,
    ) -> Result<(), RingsErrorKind>
    where
        I: FnMut() -> u8,
        O: FnMut(u8),
        E: FnMut(u8),
    {
        use Instruction::*;
        use RingsErrorKind::*;

        fn get_ring(index: u8, rings: &mut [Ring]) -> Result<&mut Ring, RingsErrorKind> {
            if index >= rings.len() as u8 {
                return Err(NonexistentRing(index));
            }

            Ok(&mut rings[index as usize])
        }

        fn get_pointer(b1: u8, b2: u8) -> usize {
            ((b1 as usize) << 8) | b2 as usize
        }

        match self {
            MKR => {
                if args[0] == 0 {
                    return Err(InvalidValue(0));
                }

                if rings.len() == 0xFF {
                    return Err(TooManyRings);
                }

                rings.push(Ring::new(args[0]));
                Ok(())
            }
            PUT => {
                get_ring(args[0], rings)?[0] = args[1];

                Ok(())
            }
            ROT => {
                get_ring(args[0], rings)?.add_offset(args[1]);

                Ok(())
            }
            SWP => {
                let r1 = get_ring(args[0], rings)?[0];
                let r2 = get_ring(args[1], rings)?[0];

                get_ring(args[0], rings)?[0] = r2;
                get_ring(args[1], rings)?[0] = r1;

                Ok(())
            }
            INP => {
                get_ring(args[0], rings)?[0] = io.0();

                Ok(())
            }
            OUT => {
                io.1(get_ring(args[0], rings)?[0]);

                Ok(())
            }
            ERR => {
                io.2(get_ring(args[0], rings)?[0]);

                Ok(())
            }
            ADD => {
                let val = get_ring(args[0], rings)?[0] as u16 + get_ring(args[1], rings)?[0] as u16;
                if val > 0xFF {
                    return Err(InvalidValue(val as i32));
                }

                get_ring(args[2], rings)?[0] = val as u8;

                Ok(())
            }
            SUB => {
                let v1 = get_ring(args[0], rings)?[0];
                let v2 = get_ring(args[1], rings)?[0];
                if v2 > v1 {
                    return Err(InvalidValue(v1 as i32 - v2 as i32));
                }

                get_ring(args[2], rings)?[0] = v1 - v2;

                Ok(())
            }
            MUL => {
                let val = get_ring(args[0], rings)?[0] as u16 * get_ring(args[1], rings)?[0] as u16;
                if val > 0xFF {
                    return Err(InvalidValue(val as i32));
                }

                get_ring(args[2], rings)?[0] = val as u8;

                Ok(())
            }
            DIV => {
                get_ring(args[2], rings)?[0] =
                    get_ring(args[0], rings)?[0] / get_ring(args[1], rings)?[0];

                Ok(())
            }
            JMP => Err(Jump(get_pointer(args[0], args[1]))),
            JEQ => {
                if get_ring(args[0], rings)?[0] != get_ring(args[1], rings)?[0] {
                    return Ok(());
                }

                Err(Jump(get_pointer(args[2], args[3])))
            }
            JGT => {
                if get_ring(args[0], rings)?[0] <= get_ring(args[1], rings)?[0] {
                    return Ok(());
                }

                Err(Jump(get_pointer(args[2], args[3])))
            }
            JLT => {
                if get_ring(args[0], rings)?[0] >= get_ring(args[1], rings)?[0] {
                    return Ok(());
                }

                Err(Jump(get_pointer(args[2], args[3])))
            }
            HLT => Err(Halt(args[0])),
        }
    }
}

impl TryFrom<&str> for Instruction {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use Instruction::*;

        Ok(match value {
            "mkr" => MKR,
            "put" => PUT,
            "rot" => ROT,
            "swp" => SWP,
            "inp" => INP,
            "out" => OUT,
            "err" => ERR,
            "add" => ADD,
            "sub" => SUB,
            "mul" => MUL,
            "div" => DIV,
            "jmp" => JMP,
            "jeq" => JEQ,
            "jgt" => JGT,
            "jlt" => JLT,
            "hlt" => HLT,
            _ => return Err(()),
        })
    }
}

impl TryFrom<u8> for Instruction {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use Instruction::*;

        Ok(match value {
            0x0 => MKR,
            0x1 => PUT,
            0x2 => ROT,
            0x3 => SWP,
            0x4 => INP,
            0x5 => OUT,
            0x6 => ERR,
            0x7 => ADD,
            0x8 => SUB,
            0x9 => MUL,
            0xA => DIV,
            0xB => JMP,
            0xC => JEQ,
            0xD => JGT,
            0xE => JLT,
            0xF => HLT,
            _ => return Err(()),
        })
    }
}

impl From<&Instruction> for u8 {
    fn from(i: &Instruction) -> Self {
        use Instruction::*;

        match i {
            MKR => 0x0,
            PUT => 0x1,
            ROT => 0x2,
            SWP => 0x3,
            INP => 0x4,
            OUT => 0x5,
            ERR => 0x6,
            ADD => 0x7,
            SUB => 0x8,
            MUL => 0x9,
            DIV => 0xA,
            JMP => 0xB,
            JEQ => 0xC,
            JGT => 0xD,
            JLT => 0xE,
            HLT => 0xF,
        }
    }
}

impl From<&Instruction> for String {
    fn from(i: &Instruction) -> Self {
        use Instruction::*;

        match i {
            MKR => "mkr",
            PUT => "put",
            ROT => "rot",
            SWP => "swp",
            INP => "inp",
            OUT => "out",
            ERR => "err",
            ADD => "add",
            SUB => "sub",
            MUL => "mul",
            DIV => "div",
            JMP => "jmp",
            JEQ => "jeq",
            JGT => "jgt",
            JLT => "jlt",
            HLT => "hlt",
        }.to_owned()
    }
}