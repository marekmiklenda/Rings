use std::fmt::Display;

use crate::{
    build::{char::CharacterReaderError, statement::StatementParserError, token::TokenizerError, AssemblerError}, vm::RuntimeError, LocalizedResult, MaybeLocalized
};

pub type RingsResult<T> = Result<T, RingsError>;
pub type LocalizedRingsResult<T> = LocalizedResult<T, RingsError>;
pub type MaybeLocalizedRingsResult<T> = MaybeLocalized<RingsResult<T>>;
#[derive(Debug)]
pub enum RingsError {
    IoError(std::io::Error),

    CharacterRead(CharacterReaderError),
    Tokenizer(TokenizerError),
    StatementParser(StatementParserError),
    Assembler(AssemblerError),
    Runtime(RuntimeError),
}

impl From<std::io::Error> for RingsError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<CharacterReaderError> for RingsError {
    fn from(value: CharacterReaderError) -> Self {
        Self::CharacterRead(value)
    }
}

impl From<TokenizerError> for RingsError {
    fn from(value: TokenizerError) -> Self {
        Self::Tokenizer(value)
    }
}

impl From<StatementParserError> for RingsError {
    fn from(value: StatementParserError) -> Self {
        Self::StatementParser(value)
    }
}

impl From<AssemblerError> for RingsError {
    fn from(value: AssemblerError) -> Self {
        Self::Assembler(value)
    }
}

impl From<RuntimeError> for RingsError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

impl Display for RingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => Display::fmt(e, f),

            Self::CharacterRead(e) => writeln!(f, "UTF-8 read error: {}", e),
            Self::Tokenizer(e) => Display::fmt(e, f),
            Self::StatementParser(e) => Display::fmt(e, f),
            Self::Assembler(e) => Display::fmt(e, f),
            Self::Runtime(e) => Display::fmt(e, f),
        }
    }
}
