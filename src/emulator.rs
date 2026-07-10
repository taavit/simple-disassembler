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
            self.gpr[index - 4] = (self.gpr[index] & 0x00FF) | ((value as u16) << 8) as u16;
        }
    }
}


#[derive(Debug, Default)]
pub struct Flags {
    pub zero: bool,
    pub direction: bool,
}

#[derive(Debug, Default)]
pub struct Screen {
    pub cursor_row: u8,
    pub cursor_col: u8,
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
    pub fn new() -> Self {
        let mut regs = Registers::default();
        regs.write16(Register16::Sp, 0xFFFE);

        Self {
            registers: regs,
            flags: Flags::default(),
            ip: 0x100,
            memory: [0u8; 64 * 1024],
            screen: Screen::default(),
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
            EffectiveAddressBase::BxSi => self.registers.read16(Register16::Bx).wrapping_add(self.registers.read16(Register16::Si)),
            EffectiveAddressBase::BxDi => self.registers.read16(Register16::Bx).wrapping_add(self.registers.read16(Register16::Di)),
            EffectiveAddressBase::BpSi => self.registers.read16(Register16::Bp).wrapping_add(self.registers.read16(Register16::Si)),
            EffectiveAddressBase::BpDi => self.registers.read16(Register16::Bp).wrapping_add(self.registers.read16(Register16::Di)),

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
    use crate::{emulator::Registers, isa::{Register8, Register16}};

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


