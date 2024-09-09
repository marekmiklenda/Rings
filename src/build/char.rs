use crate::{error::LocalizedRingsResult, Localized};

type CharacterReaderResult<T> = Result<T, CharacterReaderError>;
#[derive(Debug)]
pub enum CharacterReaderError {
    IoError(std::io::Error),
    IncompleteCharacter,
    InvalidCharacter(u32),
}

impl std::error::Error for CharacterReaderError {}

impl From<std::io::Error> for CharacterReaderError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl std::fmt::Display for CharacterReaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => std::fmt::Display::fmt(e, f),
            Self::IncompleteCharacter => write!(f, "Incomplete UTF-8 character"),
            Self::InvalidCharacter(c) => write!(f, "Invalid UTF-8 codepoint: {}", c),
        }
    }
}

pub struct CharIterator<R> {
    reader: R,
    location: Localized<()>,
    done: bool,
    read_buffer: [u8; 1],
}

impl<R> CharIterator<R>
where
    R: std::io::Read,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            location: Localized::new(()),
            done: false,
            read_buffer: [0u8; 1],
        }
    }

    fn read_byte(&mut self) -> Option<CharacterReaderResult<u8>> {
        loop {
            match self.reader.read(&mut self.read_buffer) {
                Ok(v) => {
                    if v != 0 {
                        return Some(Ok(self.read_buffer[0]));
                    }

                    return None;
                }
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::UnexpectedEof => return None,
                        std::io::ErrorKind::Interrupted => continue,
                        _ => return Some(Err(CharacterReaderError::IoError(e))),
                    };
                }
            }
        }
    }

    fn get_codepoint_length(byte: u8) -> u8 {
        match byte {
            d if d < 0x80 => 1,
            d if d < 0xE0 => 2,
            d if d < 0xF0 => 3,
            _ => 4,
        }
    }

    pub fn read_char(&mut self) -> Option<CharacterReaderResult<char>> {
        let mut codepoint: u32 = 0;
        let first_byte = match self.read_byte()? {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        let codepoint_length = Self::get_codepoint_length(first_byte);
        for i in 1..codepoint_length {
            let extended_byte = match self.read_byte() {
                Some(Ok(v)) => v,
                Some(Err(e)) => return Some(Err(e)),
                None => return Some(Err(CharacterReaderError::IncompleteCharacter)),
            } as u32;

            codepoint |= (extended_byte & 0x3F) << ((codepoint_length - i - 1) * 6);
        }

        let first_part_pattern = match codepoint_length {
            1 => 0x7Fu8,
            2 => 0x1Fu8,
            3 => 0x0Fu8,
            _ => 0x07u8,
        };

        codepoint |= ((first_byte & first_part_pattern) as u32) << ((codepoint_length - 1) * 6);
        Some(
            char::from_u32(codepoint)
                .ok_or_else(|| CharacterReaderError::InvalidCharacter(codepoint)),
        )
    }
}

impl<R> From<R> for CharIterator<R>
where
    R: std::io::Read,
{
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

impl<R> Iterator for CharIterator<R>
where
    R: std::io::Read,
{
    type Item = LocalizedRingsResult<char>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match self.read_char() {
            None => {
                self.done = true;
                Some(self.location.transform(Ok('\n')))
            }
            Some(Ok(c)) => {
                let ret = Some(self.location.transform(Ok(c)));
                if c == '\n' {
                    self.location.new_line();
                } else {
                    self.location.inc_char();
                }
                ret
            }
            Some(Err(e)) => {
                self.done = true;
                Some(self.location.transform(Err(e.into())))
            }
        }
    }
}
