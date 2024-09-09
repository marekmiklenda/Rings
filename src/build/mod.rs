use std::collections::HashMap;

use char::CharIterator;
use statement::{InstructionArg, InstructionStatement, Statement, StatementParser};
use token::Tokenizer;

use crate::{
    error::{MaybeLocalizedRingsResult, RingsError},
    instruction::{Instruction, InstructionError, InstructionPrimitive},
    Localized, MaybeLocalized,
};

pub mod char;
pub mod statement;
pub mod token;

type AssemblerResult<T> = Result<T, AssemblerError>;
#[derive(Debug)]
pub enum AssemblerError {
    DuplicateLabel(String),
    InvalidInstructionArguments(InstructionPrimitive),
    LabelNotFound(String),
    WrongNumberOfArguments {
        primitive: InstructionPrimitive,
        expected: u8,
        got: u8,
    },
    Validation(InstructionError),
}

impl std::error::Error for AssemblerError {}

impl From<InstructionError> for AssemblerError {
    fn from(value: InstructionError) -> Self {
        Self::Validation(value)
    }
}

impl std::fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateLabel(lbl) => write!(f, "Duplicate label: {}", lbl),
            Self::InvalidInstructionArguments(prim) => {
                write!(f, "Invalid instruction arguments for {:?}", prim)
            }
            Self::LabelNotFound(lbl) => write!(f, "Label not found: {}", lbl),
            Self::WrongNumberOfArguments {
                primitive,
                expected,
                got,
            } => write!(
                f,
                "Wrong number of arguments for {:?}. Expected {}, got {}",
                primitive, expected, got
            ),
            Self::Validation(e) => write!(f, "Instruction validation error: {}", e),
        }
    }
}

pub enum Program {
    Localized(Vec<Localized<Instruction>>),
    Unlocalized(Vec<Instruction>),
}

impl Program {
    fn new(preserve_location: bool) -> Self {
        if preserve_location {
            Self::Localized(Vec::new())
        } else {
            Self::Unlocalized(Vec::new())
        }
    }

    // Intentionally not pub
    fn push(&mut self, instr: Localized<Instruction>) {
        match self {
            Self::Localized(vec) => vec.push(instr),
            Self::Unlocalized(vec) => vec.push(instr.value),
        }
    }

    pub fn get(&self, index: usize) -> Option<MaybeLocalized<Instruction>> {
        match self {
            Self::Localized(l) => l.get(index).map(|v| MaybeLocalized::Localized(v.clone())),
            Self::Unlocalized(v) => v.get(index).map(|v| MaybeLocalized::General(*v)),
        }
    }
}

pub struct ProgramAssembler {
    labels: HashMap<String, usize>,
    instructions: Vec<Localized<InstructionStatement>>,
}

impl ProgramAssembler {
    fn consume_raw_statement(
        &mut self,
        statement: Statement,
        location: &Localized<()>,
    ) -> AssemblerResult<()> {
        match statement {
            Statement::Label(lbl) => {
                #[allow(clippy::map_entry)]
                if self.labels.contains_key(&lbl) {
                    Err(AssemblerError::DuplicateLabel(lbl))
                } else {
                    self.labels.insert(lbl, self.instructions.len());
                    Ok(())
                }
            }
            Statement::Instruction(i) => {
                self.instructions.push(location.transform(i));
                Ok(())
            }
        }
    }

    fn assemble_inner(self, preserve_location: bool) -> MaybeLocalized<AssemblerResult<Program>> {
        let mut out = Program::new(preserve_location);

        macro_rules! pattern_arg {
            (num $ident:ident) => {
                InstructionArg::Number($ident)
            };

            (lbl $ident:ident) => {
                InstructionArg::Label($ident)
            };
        }

        macro_rules! process_arg {
            ($location:expr, num $ident:ident) => {
                $ident
            };

            ($location:expr, lbl $ident:ident) => {{
                match self.labels.get(&$ident) {
                    Some(v) => *v,
                    None => {
                        return MaybeLocalized::Localized(
                            $location.transform(Err(AssemblerError::LabelNotFound($ident.clone()))),
                        )
                    }
                }
            }};
        }

        macro_rules! build_instr {
            ($location:expr, $primitive:expr, $tgt:ident; $($typ:tt $arg:ident),+) => {
                match ($($arg,)+) {
                    ($(pattern_arg!($typ $arg),)+) => {
                        $location.transform(Instruction::$tgt($(process_arg!($location, $typ $arg)),+))
                    }
                    _ => {
                        return MaybeLocalized::Localized($location.transform(Err(AssemblerError::InvalidInstructionArguments(
                            $primitive
                        ))));
                    }
                }
            };
        }

        for instruction_stmt in self.instructions {
            let (location, instruction_stmt) = instruction_stmt.cut();
            let instr = match instruction_stmt {
                InstructionStatement::Instruction1(prim, a) => match prim {
                    InstructionPrimitive::MKR => build_instr!(location, prim, MKR; num a),
                    InstructionPrimitive::INP => build_instr!(location, prim, INP; num a),
                    InstructionPrimitive::OUT => build_instr!(location, prim, OUT; num a),
                    InstructionPrimitive::ERR => build_instr!(location, prim, ERR; num a),
                    InstructionPrimitive::JMP => build_instr!(location, prim, JMP; lbl a),
                    InstructionPrimitive::HLT => build_instr!(location, prim, HLT; num a),
                    primitive => {
                        return MaybeLocalized::Localized(location.transform(Err(
                            AssemblerError::WrongNumberOfArguments {
                                expected: primitive.get_num_args(),
                                primitive,
                                got: 1,
                            },
                        )))
                    }
                },
                InstructionStatement::Instruction2(prim, a, b) => match prim {
                    InstructionPrimitive::PUT => build_instr!(location, prim, PUT; num a, num b),
                    InstructionPrimitive::ROT => build_instr!(location, prim, ROT; num a, num b),
                    InstructionPrimitive::SWP => build_instr!(location, prim, SWP; num a, num b),
                    primitive => {
                        return MaybeLocalized::Localized(location.transform(Err(
                            AssemblerError::WrongNumberOfArguments {
                                expected: primitive.get_num_args(),
                                primitive,
                                got: 2,
                            },
                        )))
                    }
                },
                InstructionStatement::Instruction3(prim, a, b, c) => match prim {
                    InstructionPrimitive::ADD => {
                        build_instr!(location, prim, ADD; num a, num b, num c)
                    }
                    InstructionPrimitive::SUB => {
                        build_instr!(location, prim, SUB; num a, num b, num c)
                    }
                    InstructionPrimitive::MUL => {
                        build_instr!(location, prim, MUL; num a, num b, num c)
                    }
                    InstructionPrimitive::DIV => {
                        build_instr!(location, prim, DIV; num a, num b, num c)
                    }
                    InstructionPrimitive::JEQ => {
                        build_instr!(location, prim, JEQ; num a, num b, lbl c)
                    }
                    InstructionPrimitive::JGT => {
                        build_instr!(location, prim, JGT; num a, num b, lbl c)
                    }
                    InstructionPrimitive::JLT => {
                        build_instr!(location, prim, JLT; num a, num b, lbl c)
                    }
                    primitive => {
                        return MaybeLocalized::Localized(location.transform(Err(
                            AssemblerError::WrongNumberOfArguments {
                                expected: primitive.get_num_args(),
                                primitive,
                                got: 3,
                            },
                        )))
                    }
                },
            };

            if let Err(e) = instr.validate() {
                return MaybeLocalized::Localized(location.transform(Err(e.into())));
            }

            out.push(instr);
        }

        MaybeLocalized::General(Ok(out))
    }

    pub fn assemble<R>(reader: R, preserve_location: bool) -> MaybeLocalizedRingsResult<Program>
    where
        R: std::io::Read,
    {
        let chars = CharIterator::new(reader);
        let tokens = Tokenizer::new(chars);
        let statements = StatementParser::new(tokens);

        let mut ctx = Self {
            labels: HashMap::with_capacity(50),
            instructions: Vec::new(),
        };

        for statement in statements {
            let (location, statement) = statement?.cut();
            if let Err(e) = ctx.consume_raw_statement(statement, &location) {
                return MaybeLocalized::Localized(location.transform(Err(e.into())));
            }
        }

        ctx.assemble_inner(preserve_location)
            .map(|e| e.map_err(RingsError::from))
    }
}
