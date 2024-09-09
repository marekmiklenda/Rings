use crate::{
    error::LocalizedRingsResult, instruction::InstructionPrimitive, Localized, NumberSystem,
};

type TokenizerResult<T> = Result<T, TokenizerError>;
#[derive(Debug)]
pub enum TokenizerError {
    InvalidCharacter(char),
    InvalidDigit(NumberSystem, char),
    NumberOutOfRange(NumberSystem, usize),
    UnfinishedToken,
}

impl std::error::Error for TokenizerError {}

impl std::fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCharacter(c) => write!(f, "Invalid character: {}", c),
            Self::InvalidDigit(s, d) => write!(f, "Invalid digit for number system {}: {}", s, d),
            Self::NumberOutOfRange(_, n) => write!(f, "Number out of range: {}", n),
            Self::UnfinishedToken => write!(f, "Unfinished token"),
        }
    }
}

enum WordPrimitive {
    Char(char),
    TwoChars(char, char),
    ThreeChars(char, char, char),
    String(String),
}

impl WordPrimitive {
    pub fn into_string(self) -> String {
        match self {
            Self::String(s) => s,
            s => s.to_string(),
        }
    }

    pub fn into_token(self) -> Token {
        match &self {
            Self::ThreeChars(a, b, c) => match InstructionPrimitive::try_from((*a, *b, *c)) {
                Ok(v) => Token::InstructionPrimitive(v),
                Err(..) => Token::Word(self.to_string()),
            },
            _ => Token::Word(self.into_string()),
        }
    }

    pub fn push(&mut self, char: char) {
        match self {
            Self::Char(a) => *self = Self::TwoChars(*a, char),
            Self::TwoChars(a, b) => *self = Self::ThreeChars(*a, *b, char),
            Self::ThreeChars(a, b, c) => {
                *self = Self::String({
                    let mut str = String::with_capacity(4);
                    str.push(*a);
                    str.push(*b);
                    str.push(*c);
                    str.push(char);
                    str
                })
            }
            Self::String(s) => s.push(char),
        };
    }
}

impl std::fmt::Display for WordPrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Display;
        match self {
            Self::Char(c) => Display::fmt(c, f),
            Self::TwoChars(a, b) => write!(f, "{}{}", a, b),
            Self::ThreeChars(a, b, c) => write!(f, "{}{}{}", a, b, c),
            Self::String(s) => Display::fmt(s, f),
        }
    }
}

#[derive(Debug)]
pub enum Token {
    Colon,
    Word(String),
    Number(u8),
    Newline,
    InstructionPrimitive(InstructionPrimitive),
}

#[derive(Default)]
enum TokenizerState {
    #[default]
    Init,
    Comment,
    /// A number has been detected with a leading zero
    Leading0,
    /// A number of a known system is being built, but is not yet complete
    NumberStub(usize, NumberSystem),
    /// A number of a known system is being built and can be considered complete
    Number(usize, NumberSystem),
    /// Word.. A sequence of characters
    Word(WordPrimitive),
}

pub struct Tokenizer<I> {
    state: TokenizerState,
    carry: Option<LocalizedRingsResult<Token>>,
    last_location: Localized<()>,
    src: I,
    done: bool,
}

impl<I> Tokenizer<I>
where
    I: Iterator<Item = LocalizedRingsResult<char>>,
{
    pub fn new(src: I) -> Self {
        Self {
            state: TokenizerState::default(),
            carry: None,
            src,
            done: false,
            last_location: Localized::default(),
        }
    }

    fn consume(&mut self, c: char) -> TokenizerResult<Option<Token>> {
        match &mut self.state {
            TokenizerState::Init => match c {
                '#' => {
                    self.state = TokenizerState::Comment;
                    Ok(None)
                }
                ':' => Ok(Some(Token::Colon)),
                '0' => {
                    self.state = TokenizerState::Leading0;
                    Ok(None)
                }
                '1'..='9' => {
                    self.state =
                        TokenizerState::Number(c as usize - '0' as usize, NumberSystem::Decimal);
                    Ok(None)
                }
                c if c.is_whitespace() => Ok(None),
                c => {
                    self.state = TokenizerState::Word(WordPrimitive::Char(c));
                    Ok(None)
                }
            },
            TokenizerState::Comment => match c {
                '\n' => {
                    self.state = TokenizerState::Init;
                    Ok(None)
                }
                _ => Ok(None),
            },
            TokenizerState::Leading0 => match c {
                'x' | 'X' => {
                    self.state = TokenizerState::NumberStub(0, NumberSystem::Hexadecimal);
                    Ok(None)
                }
                'b' => {
                    self.state = TokenizerState::NumberStub(0, NumberSystem::Binary);
                    Ok(None)
                }
                '0'..='7' => {
                    let num = c as usize - '0' as usize;
                    self.state = TokenizerState::Number(num, NumberSystem::Octal);
                    Ok(None)
                }
                '8' | '9' => Err(TokenizerError::InvalidDigit(NumberSystem::Octal, c)),
                c if c.is_whitespace() => {
                    self.state = TokenizerState::Init;
                    Ok(Some(Token::Number(0)))
                }
                _ => Err(TokenizerError::InvalidCharacter(c)),
            },
            TokenizerState::NumberStub(val, sys) => {
                macro_rules! num_sys {
                    ($base:literal, $range:pat) => {
                        num_sys!($base, $range => c as usize - '0' as usize)
                    };

                    ($base:literal, $(
                        $pat:pat => $expr:expr$(,)?
                    ),+) => {
                        match c {
                            $(
                                $pat => {
                                    let val = *val * $base + $expr;
                                    self.state = TokenizerState::Number(val, *sys);
                                    Ok(None)
                                }
                            )+
                            _ => Err(TokenizerError::InvalidCharacter(c)),
                        }
                    };
                }

                match sys {
                    NumberSystem::Binary => num_sys!(2, '0' | '1'),
                    NumberSystem::Octal => num_sys!(8, '0'..='7'),
                    NumberSystem::Decimal => num_sys!(10, '0'..='9'),
                    NumberSystem::Hexadecimal => num_sys!(
                        16,
                        '0'..='9' => c as usize - '0' as usize,
                        'A'..='F' => c as usize - 'A' as usize + 10,
                        'a'..='f' => c as usize - 'a' as usize + 10,
                    ),
                    NumberSystem::Unknown => unreachable!(),
                }
            }
            TokenizerState::Number(val, sys) => {
                macro_rules! num_sys {
                    ($base:literal, $range:pat) => {
                        num_sys!($base, $range => c as usize - '0' as usize)
                    };

                    ($base:literal, $(
                        $pat:pat => $expr:expr$(,)?
                    ),+) => {
                        match c {
                            $(
                                $pat => {
                                    let num = $expr;
                                    *val = *val * $base + num;
                                    Ok(None)
                                }
                            )+
                            c if c.is_whitespace() => {
                                let number = *val;
                                if number > u8::MAX as usize {
                                    return Err(TokenizerError::NumberOutOfRange(
                                        NumberSystem::Binary,
                                        number,
                                    ));
                                }
                                self.state = TokenizerState::Init;
                                Ok(Some(Token::Number(number as u8)))
                            },
                            _ => Err(TokenizerError::InvalidCharacter(c)),
                        }
                    };
                }

                match sys {
                    NumberSystem::Binary => num_sys!(2, '0' | '1'),
                    NumberSystem::Octal => num_sys!(8, '0'..='7'),
                    NumberSystem::Decimal => num_sys!(10, '0'..='9'),
                    NumberSystem::Hexadecimal => num_sys!(
                        16,
                        '0'..='9' => c as usize - '0' as usize,
                        'A'..='F' => c as usize - 'A' as usize + 10,
                        'a'..='f' => c as usize - 'a' as usize + 10,
                    ),
                    NumberSystem::Unknown => unreachable!(),
                }
            }
            TokenizerState::Word(w) => match c {
                c if c.is_whitespace() => {
                    let TokenizerState::Word(w) =
                        std::mem::replace(&mut self.state, TokenizerState::Init)
                    else {
                        unreachable!()
                    };

                    Ok(Some(w.into_token()))
                }
                c => {
                    w.push(c);
                    Ok(None)
                }
            },
        }
    }
}

impl<I> From<I> for Tokenizer<I>
where
    I: Iterator<Item = LocalizedRingsResult<char>>,
{
    fn from(value: I) -> Self {
        Self::new(value)
    }
}

impl<I> Iterator for Tokenizer<I>
where
    I: Iterator<Item = LocalizedRingsResult<char>>,
{
    type Item = LocalizedRingsResult<Token>;
    fn next(&mut self) -> Option<Self::Item> {
        {
            let carry = self.carry.take();
            if carry.is_some() {
                return carry;
            }
        }

        if self.done {
            return None;
        }

        loop {
            let localized = match self.src.next() {
                None => {
                    self.done = true;
                    return if let TokenizerState::Init = self.state {
                        None
                    } else {
                        Some(
                            self.last_location
                                .transform(Err(TokenizerError::UnfinishedToken.into())),
                        )
                    };
                }
                Some(loc) => match std::ops::Try::branch(loc) {
                    std::ops::ControlFlow::Break(v) => {
                        self.done = true;
                        return Some(std::ops::FromResidual::from_residual(v));
                    }
                    std::ops::ControlFlow::Continue(v) => v,
                },
            };

            if let TokenizerState::Init = self.state {
                self.last_location = localized.transform(());
            }

            let maybe_token = match self.consume(localized.value) {
                Ok(Some(token)) => Some(self.last_location.transform(Ok(token))),
                Ok(None) => None,
                Err(e) => {
                    self.done = true;
                    return Some(self.last_location.transform(Err(e.into())));
                }
            };

            if maybe_token.is_some() {
                if localized.value == '\n' {
                    self.carry = Some(localized.transform(Ok(Token::Newline)));
                }

                return maybe_token;
            } else if localized.value == '\n' {
                return Some(localized.transform(Ok(Token::Newline)));
            }
        }
    }
}
