use std::fmt::Display;

use crate::isa::{EffectiveAddressBase, MemSpec, Operand, Register8, Register16};

#[derive(Debug, Default)]
pub struct Registers {
    // 0: AX, 1: CX, 2: DX, 3: BX, 4: SP, 5: BP, 6: SI, 7: DI
    gpr: [u16; 8],
    cs: u16,
    ds: u16,
    es: u16,
    ss: u16,
}

impl Registers {
    pub fn read16(&self, reg: Register16) -> u16 {
        self.gpr[reg as usize & 7]
    }

    pub fn write16(&mut self, reg: Register16, value: u16) {
        self.gpr[reg as usize & 7] = value;
    }

    pub fn read8(&self, reg: Register8) -> u8 {
        let index = reg as usize & 7;
        if index < 4 {
            (self.gpr[index] & 0xFF) as u8
        } else {
            ((self.gpr[index - 4] >> 8) & 0xFF) as u8
        }
    }

    pub fn write8(&mut self, reg: Register8, value: u8) {
        let index = reg as usize & 7;
        if index < 4 {
            self.gpr[index] = (self.gpr[index] & 0xFF00) | value as u16;
        } else {
            self.gpr[index - 4] = (self.gpr[index - 4] & 0x00FF) | ((value as u16) << 8) as u16;
        }
    }
}

#[derive(Debug, Default)]
pub struct Flags {
    pub zero: bool,
    pub direction: bool,
}

fn cp437_to_unicode(byte: u8) -> char {
    match byte {
        0x00..=0x7F => byte as char,
        0xFC => '█', // blok pełny — bardzo częsty w bannerach
        0xDB => '█',
        0xB0 => '░',
        0xB1 => '▒',
        0xB2 => '▓',
        0xFE => '■',
        0xFF => '▬',
        0xF0 => '≡',
        0xF1 => '±',
        0xF2 => '≥',
        0xF3 => '≤',
        0xDA => '╔',
        0xBF => '╗',
        0xC0 => '╚',
        0xD9 => '╝',
        0xC4 => '─',
        0xCD => '═',
        0xC2 => '╦',
        0xCA => '╩',
        _ => byte as char, // fallback
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Cell {
    data: u8,
    attributes: u8,
}
const CELL_FG: [u8; 16] = [
    30, 34, 32, 36, 31, 35, 33, 37, 90, 94, 92, 96, 91, 95, 93, 97,
];

const CELL_BG: [u8; 16] = [
    40, 44, 42, 46, 41, 45, 43, 47, 100, 104, 102, 106, 101, 105, 103, 107,
];

#[derive(Debug, Default)]
pub struct Screen {
    pub width: u8,
    pub height: u8,
    pub current_row: u8,
    pub current_col: u8,
    pub data: Vec<Cell>,
    pub dirty: bool,
}

impl Screen {
    pub fn new(width: u8, height: u8) -> Screen {
        Screen {
            width,
            height,
            current_row: 0,
            current_col: 0,
            data: vec![Cell::default(); width as usize * height as usize],
            dirty: false,
        }
    }
}
impl Display for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut current_attr = None;
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.data[self.width as usize * y as usize + x as usize];
                if current_attr != Some(cell.attributes) {
                    current_attr = Some(cell.attributes);
                    write!(
                        f,
                        "\x1b[{};{}m",
                        CELL_FG[(cell.attributes & 0x0F) as usize],
                        CELL_BG[((cell.attributes & 0xF0) >> 4) as usize]
                    )?
                }
                write!(f, "{}", cp437_to_unicode(cell.data))?;
            }
            write!(f, "\r\n")?;
        }
        write!(f, "\x1b[0m")?;
        Ok(())
    }
}

impl Screen {
    pub fn clean_dirty_flag(&mut self) {
        self.dirty = false;
    }
    // Int21,9
    pub fn write_char(&mut self, ch: u8) {
        match ch {
            0x0D => {
                self.current_col = 0;
            }
            0x0A => {
                self.current_row += 1;
            }
            _ => {
                let Some(cell) = self.data.get_mut(
                    self.current_row as usize * self.width as usize + self.current_col as usize,
                ) else {
                    return;
                };
                cell.data = ch;
                self.current_col += 1;
                if self.current_col >= self.width {
                    self.current_row += 1;
                    self.current_col = 0;
                }
                self.dirty = true;
            }
        }
    }
    // Int10,2
    pub fn set_cursor_pos(&mut self, _page: u8, row: u8, col: u8) {
        self.current_col = col;
        self.current_row = row;
    }

    // Int10,9
    pub fn write_char_and_attr_at_current(
        &mut self,
        _page: u8,
        character: u8,
        attribute: u8,
        count: u16,
    ) {
        let start = self.current_row as usize * self.width as usize + self.current_col as usize;
        for i in 0..count {
            let Some(cell) = self.data.get_mut(start + i as usize) else {
                break;
            };
            cell.data = character;
            cell.attributes = attribute;
        }
        self.dirty = true;
    }
}

#[derive(Debug)]
pub struct Cpu {
    pub registers: Registers,
    pub flags: Flags,
    pub memory: [u8; 1024 * 64],
    pub ip: u16,
    pub screen: Screen,
}

impl Cpu {
    pub fn new(screen: Screen) -> Self {
        let mut regs = Registers::default();
        regs.write16(Register16::Sp, 0xFFFE);

        Self {
            registers: regs,
            flags: Flags::default(),
            ip: 0x100,
            memory: [0u8; 64 * 1024],
            screen,
        }
    }
    pub fn read_u8(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }
    pub fn read_u16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.memory[addr as usize], self.memory[addr as usize + 1]])
    }

    pub fn write_u8(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }
    pub fn write_u16(&mut self, addr: u16, value: u16) {
        let bytes = value.to_le_bytes();
        self.memory[addr as usize] = bytes[0];
        self.memory[addr as usize + 1] = bytes[1];
    }

    fn resolve_address(&self, spec: &MemSpec) -> u16 {
        if spec.is_direct {
            return spec.disp as u16;
        }
        let base_value = match spec.base {
            EffectiveAddressBase::BxSi => self
                .registers
                .read16(Register16::Bx)
                .wrapping_add(self.registers.read16(Register16::Si)),
            EffectiveAddressBase::BxDi => self
                .registers
                .read16(Register16::Bx)
                .wrapping_add(self.registers.read16(Register16::Di)),
            EffectiveAddressBase::BpSi => self
                .registers
                .read16(Register16::Bp)
                .wrapping_add(self.registers.read16(Register16::Si)),
            EffectiveAddressBase::BpDi => self
                .registers
                .read16(Register16::Bp)
                .wrapping_add(self.registers.read16(Register16::Di)),

            EffectiveAddressBase::Bx => self.registers.read16(Register16::Bx),
            EffectiveAddressBase::Di => self.registers.read16(Register16::Di),
            EffectiveAddressBase::Si => self.registers.read16(Register16::Si),
            EffectiveAddressBase::Bp => self.registers.read16(Register16::Bp),
            EffectiveAddressBase::None => 0,
        };

        base_value.wrapping_add(spec.disp as u16)
    }

    pub fn get_operand_value(&self, operand: &Operand) -> u16 {
        match operand {
            Operand::Register8(reg) => self.registers.read8(*reg) as u16,
            Operand::Register16(reg) => self.registers.read16(*reg) as u16,
            Operand::Imm8(val) => *val as u16,
            Operand::Imm16(val) => *val,
            Operand::RelAddress(val) => *val,
            Operand::Mem8(spec) => self.read_u8(self.resolve_address(spec)) as u16,
            Operand::Mem16(spec) => self.read_u16(self.resolve_address(spec)),
        }
    }

    pub fn set_operand_value(&mut self, operand: &Operand, value: u16) {
        match operand {
            Operand::Register8(reg) => self.registers.write8(*reg, value as u8),
            Operand::Register16(reg) => self.registers.write16(*reg, value),
            Operand::Mem8(spec) => self.write_u8(self.resolve_address(spec), value as u8),
            Operand::Mem16(spec) => self.write_u16(self.resolve_address(spec), value),
            _ => panic!("Operand read only!"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        emulator::Registers,
        isa::{Register8, Register16},
    };

    #[test]
    fn test_register_read_write() {
        let mut reg = Registers::default();

        reg.write8(Register8::Ah, 0x12);
        reg.write8(Register8::Al, 0x34);
        assert_eq!(reg.read8(Register8::Ah), 0x12);
        assert_eq!(reg.read8(Register8::Al), 0x34);
        assert_eq!(reg.read16(Register16::Ax), 0x1234);

        reg.write8(Register8::Bh, 0x56);
        reg.write8(Register8::Bl, 0x78);
        assert_eq!(reg.read8(Register8::Bh), 0x56);
        assert_eq!(reg.read8(Register8::Bl), 0x78);
        assert_eq!(reg.read16(Register16::Bx), 0x5678);
    }
}
