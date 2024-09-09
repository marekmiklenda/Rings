use std::ops::{Index, IndexMut};

use crate::{build::Program, error::MaybeLocalizedRingsResult, io::RingsIo, MaybeLocalized};

pub(crate) type RuntimeResult<T> = Result<T, RuntimeError>;
#[derive(Debug)]
pub enum RuntimeError {
    InvalidRing(RingId),
    ZeroRingSize,
}

impl std::error::Error for RuntimeError {}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRing(i) => write!(f, "Invalid ring {}", i),
            Self::ZeroRingSize => write!(f, "Attempting to create a ring with a zero size"),
        }
    }
}

pub type RingId = u8;
pub struct Ring {
    rotation_offset: u8,
    values: Vec<u8>,
}

impl Ring {
    pub fn new(capacity: u8) -> RuntimeResult<Self> {
        if capacity == 0 {
            return Err(RuntimeError::ZeroRingSize);
        }

        Ok(Self {
            rotation_offset: 0,
            values: vec![0; capacity as usize],
        })
    }

    fn get_absolute_index(&self, relative: u8) -> u8 {
        ((self.rotation_offset as u16 + self.len() as u16 - (relative % self.len()) as u16)
            % self.values.len() as u16) as u8
    }

    pub fn rotate(&mut self, by: u8) {
        self.rotation_offset = self.rotation_offset.wrapping_add(by);
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u8 {
        self.values.len() as u8
    }

    pub fn current(&self) -> &u8 {
        &self[0]
    }

    pub fn current_mut(&mut self) -> &mut u8 {
        &mut self[0]
    }
}

impl Index<u8> for Ring {
    type Output = u8;
    fn index(&self, index: u8) -> &Self::Output {
        &self.values[self.get_absolute_index(index) as usize]
    }
}

impl IndexMut<u8> for Ring {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        let absolute = self.get_absolute_index(index) as usize;
        &mut self.values[absolute]
    }
}

impl std::fmt::Display for Ring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[(+{:02X})", self.rotation_offset)?;

        for val in self.values.iter() {
            write!(f, " {:02X}", val)?;
        }

        write!(f, "]")
    }
}

pub type ExitCode = u8;
#[derive(Default)]
pub struct RingsVM {
    pub rings: Vec<Ring>,
    pub pc: usize,
    pub exit_code: Option<ExitCode>,
}

impl RingsVM {
    pub(crate) fn get_ring(&mut self, index: RingId) -> RuntimeResult<&mut Ring> {
        self.rings
            .get_mut(index as usize)
            .ok_or(RuntimeError::InvalidRing(index))
    }

    pub fn execute<I>(program: &Program, io: &mut I) -> MaybeLocalizedRingsResult<ExitCode>
    where
        I: RingsIo,
    {
        let mut vm = Self::default();

        let exit_code = loop {
            let Some(instr) = program.get(vm.pc) else {
                break 0;
            };

            vm.pc += 1;
            //println!("{:?}", instr);

            if let Err(e) = instr.execute(&mut vm, io) {
                return instr.transform(Err(e.into()));
            }

            /*if let Ok(ring) = vm.get_ring(0) {
                println!("{}", ring);
            }
            if let Ok(ring) = vm.get_ring(1) {
                println!("{}", ring);
            }*/

            if let Some(exit_code) = vm.exit_code {
                break exit_code;
            }
        };

        MaybeLocalized::General(Ok(exit_code))
    }
}
