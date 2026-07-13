use std::fmt::Display;

use crate::isa::{EffectiveAddressBase, MemSpec, Op, Operand, Register8, Register16};

#[derive(Debug, Default)]
pub struct Registers {
    // 0: AX, 1: CX, 2: DX, 3: BX, 4: SP, 5: BP, 6: SI, 7: DI
    gpr: [u16; 8],
    _cs: u16,
    _ds: u16,
    _es: u16,
    _ss: u16,

    ip: u16,
}

impl Registers {
    pub fn step_ip(&mut self) {
        self.ip = self.ip.wrapping_add(1);
    }

    pub fn set_ip(&mut self, ip: u16) {
        self.ip = ip;
    }

    pub fn step_ip_by(&mut self, v: u16) {
        self.ip = self.ip.wrapping_add(v);
    }

    pub fn ip(&self) -> u16 {
        self.ip
    }
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
    pub overflow: bool,
    pub interrupt: bool,
    pub trap: bool,
    pub sign_flag: bool,
    pub parity: bool,
    pub carry: bool,
    pub half_carry: bool,
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
pub struct VideoCard {
    pub width: u8,
    pub height: u8,
    pub current_row: u8,
    pub current_col: u8,
    pub data: Vec<Cell>,
    pub dirty: bool,
}

impl VideoCard {
    pub fn new(width: u8, height: u8) -> VideoCard {
        VideoCard {
            width,
            height,
            current_row: 0,
            current_col: 0,
            data: vec![Cell::default(); width as usize * height as usize],
            dirty: false,
        }
    }
}
impl Display for VideoCard {
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

impl VideoCard {
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
    pub memory: [u8; 1024 * 1024],
    pub screen: VideoCard,
}

impl Cpu {
    pub fn new(screen: VideoCard) -> Self {
        let mut regs = Registers::default();
        regs.ip = 0x100;
        regs.write16(Register16::Sp, 0xFFFE);

        Self {
            registers: regs,
            flags: Flags::default(),
            memory: [0u8; 1024 * 1024],
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
            Operand::Register16(reg) => self.registers.read16(*reg),
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

    pub fn execute(&mut self, instruction: Op) -> bool {
        match instruction {
            Op::Nop => {}
            Op::Mov { src, dst } => {
                let value = self.get_operand_value(&src);
                self.set_operand_value(&dst, value);
            }
            Op::Xor { src, dst } => {
                let v1 = self.get_operand_value(&src);
                let v2 = self.get_operand_value(&dst);
                let r = v1 ^ v2;
                self.flags.zero = r == 0;
                self.set_operand_value(&dst, r);
            }
            Op::PushCs => {
                let curret_sp = self.registers.read16(Register16::Sp);
                let new_sp = curret_sp.wrapping_sub(2);
                self.registers.write16(Register16::Sp, new_sp);
                let cs_val = 0u16;
                self.write_u16(new_sp, cs_val);
            }
            Op::PopDs => {
                let curret_sp = self.registers.read16(Register16::Sp);
                let _val = self.read_u16(curret_sp);
                self.registers
                    .write16(Register16::Sp, curret_sp.wrapping_add(2));
            }
            Op::Cld => {
                self.flags.direction = false;
            }
            Op::Std => {
                self.flags.direction = true;
            }
            Op::Lodsw => {
                let si_address = self.registers.read16(Register16::Si);
                let val = self.read_u16(si_address);
                self.registers.write16(Register16::Ax, val);
                self.registers.write16(
                    Register16::Si,
                    if self.flags.direction {
                        si_address.wrapping_sub(2)
                    } else {
                        si_address.wrapping_add(2)
                    },
                );
            }
            Op::Xchg { src, dst } => {
                let v1 = self.get_operand_value(&src);
                let v2 = self.get_operand_value(&dst);
                self.set_operand_value(&src, v2);
                self.set_operand_value(&dst, v1);
            }
            Op::PushReg16(reg) => {
                let curret_sp = self.registers.read16(Register16::Sp);
                let new_sp = curret_sp.wrapping_sub(2);
                self.registers.write16(Register16::Sp, new_sp);
                let val = self.registers.read16(reg);
                self.write_u16(new_sp, val);
            }
            Op::PopReg16(reg) => {
                let curret_sp = self.registers.read16(Register16::Sp);
                let value = self.read_u16(curret_sp);
                self.registers
                    .write16(Register16::Sp, curret_sp.wrapping_add(2));
                self.registers.write16(reg, value);
            }
            Op::Int(0x10) => {
                let ah = self.registers.read8(Register8::Ah);
                match ah {
                    0x02 => {
                        let page = self.registers.read8(Register8::Bh);
                        let row = self.registers.read8(Register8::Dh);
                        let col = self.registers.read8(Register8::Dl);
                        self.screen.set_cursor_pos(page, row, col);
                    }
                    0x03 => {
                        self.registers.write8(Register8::Ch, 0);
                        self.registers.write8(Register8::Cl, 15);
                        self.registers
                            .write8(Register8::Dh, self.screen.current_row);
                        self.registers
                            .write8(Register8::Dl, self.screen.current_col);
                    }
                    0x09 => {
                        let character = self.registers.read8(Register8::Al);
                        let page = self.registers.read8(Register8::Bh);
                        let attribute = self.registers.read8(Register8::Bl);
                        let count = self.registers.read16(Register16::Cx);

                        self.screen
                            .write_char_and_attr_at_current(page, character, attribute, count);
                    }
                    0x0F => {
                        self.registers.write8(Register8::Al, 3);
                        self.registers.write8(Register8::Ah, 80);
                        self.registers.write8(Register8::Bh, 0);
                    }
                    _ => {
                        panic!("Unhandled 0x10:{:02X}", ah)
                    }
                }
            }
            Op::Int(0x21) => {
                let ah = self.registers.read8(Register8::Ah);
                match ah {
                    0x02 => {
                        let data = self.registers.read8(Register8::Dl);
                        self.screen.write_char(data);
                    }
                    0x09 => {
                        let mut addr = self.registers.read16(Register16::Dx);
                        loop {
                            let ch = self.read_u8(addr);
                            if ch == b'$' {
                                break;
                            }

                            self.screen.write_char(ch);
                            addr = addr.wrapping_add(1);
                        }
                    }
                    0x30 => {
                        self.registers.write8(Register8::Al, 3);
                        self.registers.write8(Register8::Ah, 30);
                        self.registers.write8(Register8::Bh, 0);
                        self.registers.write8(Register8::Bl, 0);
                        self.registers.write16(Register16::Cx, 0);
                    }
                    0x4C => {
                        let _exit_status = self.registers.read8(Register8::Al);
                        return false;
                    }
                    _ => {
                        panic!("Unhandled 0x21:{:02X}", ah)
                    }
                }
            }
            Op::Cmp { src, dst } => {
                let src_val = self.get_operand_value(&src);
                let dst_val = self.get_operand_value(&dst);

                let (res, _overflow) = dst_val.overflowing_sub(src_val);
                self.flags.zero = res == 0;
            }
            Op::Test { src, dst } => {
                let src_val = self.get_operand_value(&src);
                let dst_val = self.get_operand_value(&dst);

                let res = dst_val & src_val;
                self.flags.zero = res == 0;
            }
            Op::Jnz(target) => {
                let dest = self.get_operand_value(&target);
                if !self.flags.zero {
                    self.registers.set_ip(dest);
                }
            }
            Op::Jz(target) => {
                let dest = self.get_operand_value(&target);
                if self.flags.zero {
                    self.registers.set_ip(dest);
                }
            }
            Op::Jmp(target) => {
                let dest = self.get_operand_value(&target);
                self.registers.set_ip(dest);
            }
            Op::Dec(operand) => {
                let v = self.get_operand_value(&operand);
                self.set_operand_value(&operand, v.wrapping_sub(1));
            }
            Op::Inc(operand) => {
                let v = self.get_operand_value(&operand);
                self.set_operand_value(&operand, v.wrapping_add(1));
            }
            Op::Shl { src, dst } => {
                let src_value = self.get_operand_value(&src);
                let dst_value = self.get_operand_value(&dst);
                self.set_operand_value(&dst, dst_value.wrapping_shl(src_value as u32));
            }
            Op::Shr { src, dst } => {
                let src_value = self.get_operand_value(&src);
                let dst_value = self.get_operand_value(&dst);
                self.set_operand_value(&dst, dst_value.wrapping_shr(src_value as u32));
            }
            Op::Add { src, dst } => {
                let src_val = self.get_operand_value(&src);
                let dst_val = self.get_operand_value(&dst);
                let result = src_val.wrapping_add(dst_val);
                self.set_operand_value(&dst, result);
                self.flags.zero = result == 0;
            }
            Op::Ret => {
                let ip = self.pop_u16();
                self.registers.set_ip(ip);
            }
            Op::Call { target } => {
                let dest = self.get_operand_value(&target);
                self.push_u16(self.registers.ip());
                self.registers.set_ip(dest);
            }
            instruction => {
                println!("[EMU][ERROR] Unknown instruction: {instruction}.");
                return false;
            }
        }

        true
    }

    fn pop_u16(&mut self) -> u16 {
        let curret_sp = self.registers.read16(Register16::Sp);
        let value = self.read_u16(curret_sp);
        self.registers
            .write16(Register16::Sp, curret_sp.wrapping_add(2));
        value
    }

    fn push_u16(&mut self, val: u16) {
        let curret_sp = self.registers.read16(Register16::Sp);
        let new_sp = curret_sp.wrapping_sub(2);
        self.registers.write16(Register16::Sp, new_sp);
        self.write_u16(new_sp, val);
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
