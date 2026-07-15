use crate::{
    emulator::{Cpu, Machine},
    isa::{Register8, Register16},
};

pub struct Dos;

impl Dos {
    const DOS_VERSION_MAJOR: u8 = 3;
    const DOS_VERSION_MINOR: u8 = 30;

    pub fn handle_interrupt(interrupt: u8, cpu: &mut Cpu, machine: &mut Machine) -> bool {
        match interrupt {
            0x21 => {
                let ah = cpu.registers.read8(Register8::Ah);
                match ah {
                    0x02 => {
                        let data = cpu.registers.read8(Register8::Dl);
                        machine.screen.write_char(data);
                    }
                    0x09 => {
                        let mut addr = cpu.registers.read16(Register16::Dx);
                        loop {
                            let ch = machine.read_u8(addr as u32);
                            if ch == b'$' {
                                break;
                            }

                            machine.screen.write_char(ch);
                            addr = addr.wrapping_add(1);
                        }
                    }
                    0x30 => {
                        cpu.registers.write8(Register8::Al, Self::DOS_VERSION_MAJOR);
                        cpu.registers.write8(Register8::Ah, Self::DOS_VERSION_MINOR);
                        cpu.registers.write8(Register8::Bh, 0);
                        cpu.registers.write8(Register8::Bl, 0);
                        cpu.registers.write16(Register16::Cx, 0);
                    }
                    0x4C => {
                        let _exit_status = cpu.registers.read8(Register8::Al);
                        return false;
                    }
                    _ => {
                        panic!("Unhandled 0x21:{:02X}", ah)
                    }
                }
            }
            int => {
                panic!("Unhandled interrupt:{:02X}", int);
            }
        }

        true
    }
}
