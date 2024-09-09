use byteorder::{ReadBytesExt, WriteBytesExt};

use crate::vm::RingsVM;

pub trait RingsIo {
    fn inp(&mut self, vm: &RingsVM) -> u8;

    fn out(&mut self, value: u8, vm: &RingsVM);

    fn err(&mut self, value: u8, vm: &RingsVM);
}

pub struct SystemStdio;

impl RingsIo for SystemStdio {
    fn out(&mut self, value: u8, _vm: &RingsVM) {
        let _ = std::io::stdout().write_u8(value);
    }

    fn inp(&mut self, _vm: &RingsVM) -> u8 {
        std::io::stdin().read_u8().unwrap_or(0xFF)
    }

    fn err(&mut self, value: u8, _vm: &RingsVM) {
        let _ = std::io::stderr().write_u8(value);
    }
}
