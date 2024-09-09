use crate::{error::LocalizedRingsResult, instruction::InstructionPrimitive, Localized};

use super::token::Token;

type StatementParserResult<T> = Result<T, StatementParserError>;
#[derive(Debug)]
pub enum StatementParserError {
    UnexpectedToken(Token),
    /// Label (Token::Word) was expected, instead got self.0
    LabelExpected(Token),
    /// Instr arg (Token::Colon | Token::Number) was expected, instead got self.0
    InstrArgExpected(Token),
    UnclosedStatement,
}

impl std::error::Error for StatementParserError {}

impl std::fmt::Display for StatementParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedToken(token) => write!(f, "Unexpected token: {:?}", token),
            Self::LabelExpected(instead) => write!(f, "Expected label, got {:?}", instead),
            Self::InstrArgExpected(instead) => {
                write!(f, "Expected instruction argument, got {:?}", instead)
            }
            Self::UnclosedStatement => write!(f, "Unclosed statement"),
        }
    }
}

#[derive(Debug)]
pub enum InstructionArg {
    Number(u8),
    Label(String),
}

#[derive(Default, Clone, Copy)]
enum InstructionArgBuilder {
    #[default]
    Empty,
    Colon,
}

impl InstructionArgBuilder {
    pub fn push(&mut self, token: Token) -> StatementParserResult<Option<InstructionArg>> {
        match self {
            Self::Empty => match token {
                Token::Colon => {
                    *self = InstructionArgBuilder::Colon;
                    Ok(None)
                }
                Token::Number(n) => Ok(Some(InstructionArg::Number(n))),
                token => Err(StatementParserError::InstrArgExpected(token)),
            },
            Self::Colon => match token {
                Token::Word(w) => {
                    *self = InstructionArgBuilder::Empty;
                    Ok(Some(InstructionArg::Label(w)))
                }
                token => Err(StatementParserError::LabelExpected(token)),
            },
        }
    }
}

#[derive(Debug)]
pub enum InstructionStatement {
    Instruction1(InstructionPrimitive, InstructionArg),
    Instruction2(InstructionPrimitive, InstructionArg, InstructionArg),
    Instruction3(
        InstructionPrimitive,
        InstructionArg,
        InstructionArg,
        InstructionArg,
    ),
}

#[derive(Debug)]
pub enum Statement {
    Label(String),
    Instruction(InstructionStatement),
}

#[derive(Default)]
enum StatementParserState {
    #[default]
    Init,
    /// A colon has been detected
    LabelStart,
    /// Instruction primitive has been detected
    InstrStart(InstructionPrimitive, InstructionArgBuilder),
    /// First argument down
    Instr1Arg(InstructionPrimitive, InstructionArg, InstructionArgBuilder),
    /// Two arguments down
    Instr2Arg(
        InstructionPrimitive,
        InstructionArg,
        InstructionArg,
        InstructionArgBuilder,
    ),
}

pub struct StatementParser<I> {
    tokens: I,
    done: bool,
    state: StatementParserState,
    last_location: Localized<()>,
}

impl<I> StatementParser<I>
where
    I: Iterator<Item = LocalizedRingsResult<Token>>,
{
    pub fn new(inner: I) -> Self {
        Self {
            tokens: inner,
            done: false,
            state: StatementParserState::default(),
            last_location: Localized::default(),
        }
    }

    fn consume(&mut self, token: Token) -> StatementParserResult<Option<Statement>> {
        match &mut self.state {
            StatementParserState::Init => match token {
                Token::Newline => Ok(None),
                Token::Colon => {
                    self.state = StatementParserState::LabelStart;
                    Ok(None)
                }
                Token::InstructionPrimitive(instr) => {
                    self.state =
                        StatementParserState::InstrStart(instr, InstructionArgBuilder::default());
                    Ok(None)
                }
                token => Err(StatementParserError::UnexpectedToken(token)),
            },
            StatementParserState::LabelStart => match token {
                Token::Word(w) => {
                    self.state = StatementParserState::Init;
                    Ok(Some(Statement::Label(w)))
                }
                token => Err(StatementParserError::LabelExpected(token)),
            },
            StatementParserState::InstrStart(instr, builder) => {
                if let Some(arg) = builder.push(token)? {
                    if instr.get_num_args() == 1 {
                        let instr = *instr;
                        self.state = StatementParserState::Init;
                        return Ok(Some(Statement::Instruction(
                            InstructionStatement::Instruction1(instr, arg),
                        )));
                    }

                    self.state = StatementParserState::Instr1Arg(
                        *instr,
                        arg,
                        InstructionArgBuilder::default(),
                    );
                }

                Ok(None)
            }
            StatementParserState::Instr1Arg(_, _, builder) => {
                if let Some(arg) = builder.push(token)? {
                    let StatementParserState::Instr1Arg(instr, arg_1, _) =
                        std::mem::replace(&mut self.state, StatementParserState::Init)
                    else {
                        unreachable!();
                    };

                    if instr.get_num_args() == 2 {
                        return Ok(Some(Statement::Instruction(
                            InstructionStatement::Instruction2(instr, arg_1, arg),
                        )));
                    }

                    self.state = StatementParserState::Instr2Arg(
                        instr,
                        arg_1,
                        arg,
                        InstructionArgBuilder::default(),
                    );
                }

                Ok(None)
            }
            StatementParserState::Instr2Arg(_, _, _, builder) => {
                if let Some(arg) = builder.push(token)? {
                    let StatementParserState::Instr2Arg(instr, arg_1, arg_2, _) =
                        std::mem::replace(&mut self.state, StatementParserState::Init)
                    else {
                        unreachable!();
                    };

                    return Ok(Some(Statement::Instruction(
                        InstructionStatement::Instruction3(instr, arg_1, arg_2, arg),
                    )));
                }

                Ok(None)
            }
        }
    }
}

impl<I> Iterator for StatementParser<I>
where
    I: Iterator<Item = LocalizedRingsResult<Token>>,
{
    type Item = LocalizedRingsResult<Statement>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            let (localized, token) = match self.tokens.next() {
                None => {
                    self.done = true;
                    match self.state {
                        StatementParserState::Init => return None,
                        _ => {
                            return Some(
                                self.last_location
                                    .transform(Err(StatementParserError::UnclosedStatement.into())),
                            )
                        }
                    }
                }
                Some(loc) => match std::ops::Try::branch(loc) {
                    std::ops::ControlFlow::Break(v) => {
                        self.done = true;
                        return Some(std::ops::FromResidual::from_residual(v));
                    }
                    std::ops::ControlFlow::Continue(v) => v,
                },
            }
            .cut();

            if let StatementParserState::Init = self.state {
                self.last_location = localized;
            }

            match self.consume(token) {
                Ok(Some(statement)) => return Some(self.last_location.transform(Ok(statement))),
                Ok(None) => (),
                Err(e) => {
                    self.done = true;
                    return Some(self.last_location.transform(Err(e.into())));
                }
            };
        }
    }
}
