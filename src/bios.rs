use crate::{
    emulator::{Cpu, Machine},
    isa::{Register8, Register16},
};

pub struct Bios;

impl Bios {
    pub fn handle_interrupt(interrupt: u8, cpu: &mut Cpu, machine: &mut Machine) -> bool {
        match interrupt {
            0x10 => {
                let ah = cpu.registers.read8(Register8::Ah);
                match ah {
                    0x02 => {
                        let page = cpu.registers.read8(Register8::Bh);
                        let row = cpu.registers.read8(Register8::Dh);
                        let col = cpu.registers.read8(Register8::Dl);
                        machine.screen.set_cursor_pos(page, row, col);
                    }
                    0x03 => {
                        cpu.registers.write8(Register8::Ch, 0);
                        cpu.registers.write8(Register8::Cl, 15);
                        cpu.registers
                            .write8(Register8::Dh, machine.screen.current_row);
                        cpu.registers
                            .write8(Register8::Dl, machine.screen.current_col);
                    }
                    0x09 => {
                        let character = cpu.registers.read8(Register8::Al);
                        let page = cpu.registers.read8(Register8::Bh);
                        let attribute = cpu.registers.read8(Register8::Bl);
                        let count = cpu.registers.read16(Register16::Cx);

                        machine
                            .screen
                            .write_char_and_attr_at_current(page, character, attribute, count);
                    }
                    0x0F => {
                        cpu.registers.write8(Register8::Al, 3);
                        cpu.registers.write8(Register8::Ah, 80);
                        cpu.registers.write8(Register8::Bh, 0);
                    }
                    _ => {
                        panic!("Unhandled 0x10:{:02X}", ah)
                    }
                }
            }
            interrupt => {
                panic!("Unhandled interrupt {:02X}", interrupt)
            }
        }

        true
    }
}
