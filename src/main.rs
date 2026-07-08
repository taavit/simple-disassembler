use std::{fmt::Display, io::Read, os::unix::fs::MetadataExt};

fn main() {
    let mut file = std::fs::File::open("dos/FDBANNER.COM").expect("Failed to open file");
    let size = file.metadata().unwrap().size();
    let mut buf = vec![0u8; size as usize + 0x100];
    file.read_exact(&mut buf[0x100..])
        .expect("Failed to read file");
    let mut decoder = Decoder {
        ip: 0x100,
        memory: &buf,
    };
    while decoder.ip < buf.len() as u16 {
        let ins = decode(&mut decoder);
        println!("{:04X}: {}", ins.address, ins.text);
    }
}
struct Instruction {
    address: u16,
    size: u8,
    text: String,
}

pub enum Register16 {
    Ax,
    Cx,
    Dx,
    Bx,
    Sp,
    Bp,
    Si,
    Di,
}

impl Display for Register16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Register16::Ax => write!(f, "ax"),
            Register16::Cx => write!(f, "cx"),
            Register16::Dx => write!(f, "dx"),
            Register16::Bx => write!(f, "bx"),
            Register16::Sp => write!(f, "sp"),
            Register16::Bp => write!(f, "bp"),
            Register16::Si => write!(f, "si"),
            Register16::Di => write!(f, "di"),
        }
    }
}

impl From<u8> for Register16 {
    fn from(value: u8) -> Self {
        match value & 7 {
            0 => Self::Ax,
            1 => Self::Cx,
            2 => Self::Dx,
            3 => Self::Bx,
            4 => Self::Sp,
            5 => Self::Bp,
            6 => Self::Si,
            7 => Self::Di,
            _ => unreachable!(),
        }
    }
}

pub enum Register8 {
    Al,
    Cl,
    Dl,
    Bl,
    Ah,
    Ch,
    Dh,
    Bh,
}

impl Display for Register8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Register8::Al => write!(f, "al"),
            Register8::Cl => write!(f, "cl"),
            Register8::Dl => write!(f, "dl"),
            Register8::Bl => write!(f, "bl"),
            Register8::Ah => write!(f, "ah"),
            Register8::Ch => write!(f, "ch"),
            Register8::Dh => write!(f, "dh"),
            Register8::Bh => write!(f, "bh"),
        }
    }
}

impl From<u8> for Register8 {
    fn from(value: u8) -> Self {
        match value & 7 {
            0 => Self::Al,
            1 => Self::Cl,
            2 => Self::Dl,
            3 => Self::Bl,
            4 => Self::Ah,
            5 => Self::Ch,
            6 => Self::Dh,
            7 => Self::Bh,
            _ => unreachable!(),
        }
    }
}

struct Decoder<'a> {
    memory: &'a [u8],
    ip: u16,
}

impl<'a> Decoder<'a> {
    pub fn read_u16(&mut self) -> u16 {
        let r = u16::from_le_bytes([
            self.memory[self.ip as usize],
            self.memory[self.ip as usize + 1],
        ]);
        self.ip += 2;

        r
    }

    pub fn read_u8(&mut self) -> u8 {
        let v = self.memory[self.ip as usize];
        self.ip += 1;

        v
    }

    pub fn read_rel8(&mut self) -> u16 {
        let offset = self.read_u8() as i8;
        (self.ip as i32 + offset as i32) as u16
    }

    pub fn read_rel16(&mut self) -> u16 {
        let offset = self.read_u16() as i16;
        (self.ip as i32 + offset as i32) as u16
    }
}

fn decode_modrm(byte: u8) -> (u8, u8, u8) {
    let mode = (byte >> 6) & 3;
    let reg = (byte >> 3) & 7;
    let rm = byte & 7;

    (mode, reg, rm)
}

pub enum EffectiveAddressBase {
    BxSi,
    BxDi,
    BpSi,
    BpDi,
    Si,
    Di,
    Bp,
    Bx,
}

impl Display for EffectiveAddressBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BxSi => write!(f, "bx+si"),
            Self::BxDi => write!(f, "bx+di"),
            Self::BpSi => write!(f, "bp+si"),
            Self::BpDi => write!(f, "bp+di"),
            Self::Si => write!(f, "si"),
            Self::Di => write!(f, "di"),
            Self::Bp => write!(f, "bp"),
            Self::Bx => write!(f, "bx"),
        }
    }
}

impl From<u8> for EffectiveAddressBase {
    fn from(value: u8) -> Self {
        match value & 7 {
            0 => Self::BxSi,
            1 => Self::BxDi,
            2 => Self::BpSi,
            3 => Self::BpDi,
            4 => Self::Si,
            5 => Self::Di,
            6 => Self::Bp,
            7 => Self::Bx,
            _ => unreachable!(),
        }
    }
}

pub enum Operand {
    Register8(Register8),
    Register16(Register16),
}

fn decode_rm8(decoder: &mut Decoder, mode: u8, rm: u8) -> String {
    match mode {
        0b00 => {
            if rm == 6 {
                let addr = decoder.read_u16();
                format!("[{:04X}h]", addr)
            } else {
                format!("[{}]", EffectiveAddressBase::from(rm))
            }
        }
        0b01 => {
            let disp = decoder.read_u8() as i8;
            format!("[{}+{}]", EffectiveAddressBase::from(rm), disp)
        }
        0b10 => {
            let disp = decoder.read_u16();
            format!("[{}+{:04X}h]", EffectiveAddressBase::from(rm), disp)
        }
        0b11 => Register8::from(rm).to_string(),
        _ => unreachable!(),
    }
}

fn decode_rm16(decoder: &mut Decoder, mode: u8, rm: u8) -> String {
    match mode {
        0b00 => {
            if rm == 6 {
                let addr = decoder.read_u16();
                format!("[{:04X}h]", addr)
            } else {
                format!("[{}]", EffectiveAddressBase::from(rm))
            }
        }
        0b01 => {
            let disp = decoder.read_u8() as i8;
            format!("[{}+{}]", EffectiveAddressBase::from(rm), disp)
        }
        0b10 => {
            let disp = decoder.read_u16();
            format!("[{}+{:04X}h]", EffectiveAddressBase::from(rm), disp)
        }
        0b11 => Register16::from(rm).to_string(),
        _ => unreachable!(),
    }
}

fn decode(decoder: &mut Decoder) -> Instruction {
    let address = decoder.ip;
    let opcode = decoder.read_u8();
    let text = match opcode {
        0x06 => "push es".into(),
        0x07 => "pop es".into(),
        0x0E => "push cs".into(),
        0xE9 => format!("jmp {:04X}h", decoder.read_rel16()),
        0xAD => "lodsw".into(),
        0x1F => "pop ds".into(),
        0xFC => "cld".into(),
        0xD0 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = decode_rm8(decoder, mode, rm);
            match reg {
                0 => format!("rol {},1", dst),
                1 => format!("ror {},1", dst),
                2 => format!("rcl {},1", dst),
                3 => format!("rcr {},1", dst),
                4 => format!("shl {},1", dst),
                5 => format!("shr {},1", dst),
                6 => "(invalid)".into(),
                7 => format!("sar {},1", dst),
                _ => unreachable!(),
            }
        }
        0x30 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let src = Register8::from(reg);
            let dst = decode_rm8(decoder, mode, rm);

            format!("xor {},{}", dst, src)
        }

        0x31 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let src = Register16::from(reg);
            let dst = decode_rm16(decoder, mode, rm);

            format!("xor {},{}", dst, src)
        }
        0x88 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let src = Register8::from(reg);
            let dst = decode_rm8(decoder, mode, rm);

            format!("mov {},{}", dst, src)
        }
        0x40..=0x47 => {
            let reg = opcode & 7;
            format!("inc {}", Register16::from(reg))
        }
        0xFE => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let operand = decode_rm8(decoder, mode, rm);
            match reg {
                0 => format!("inc {}", operand),
                1 => format!("dec {}", operand),
                _ => "db FEh".into(),
            }
        }
        0x80 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = decode_rm8(decoder, mode, rm);
            let imm = decoder.read_u8();

            match reg {
                0 => format!("add {},{:02X}", dst, imm),
                1 => format!("or {},{:02X}", dst, imm),
                2 => format!("adc {},{:02X}", dst, imm),
                3 => format!("sbb {},{:02X}", dst, imm),
                4 => format!("and {},{:02X}", dst, imm),
                5 => format!("sub {},{:02X}", dst, imm),
                6 => format!("xor {},{:02X}", dst, imm),
                7 => format!("cmp {},{:02X}", dst, imm),
                _ => unreachable!(),
            }
        }
        0xCD => {
            let int_num = decoder.read_u8();
            format!("int {:02X}h", int_num)
        }
        0x84 => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let src = Register8::from(reg);
            let dst = decode_rm8(decoder, mode, rm);

            format!("test {},{}", dst, src)
        }
        0x74 => format!("jz {:04X}h", decoder.read_rel8()),
        0x75 => format!("jnz {:04X}h", decoder.read_rel8()),
        0xEB => format!("jmp {:04X}h", decoder.read_rel8()),
        0x86 => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let src = Register8::from(reg);
            let dst = decode_rm8(decoder, mode, rm);

            format!("xchg {},{}", dst, src)
        }
        0x8A => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let dst = Register8::from(reg);
            let src = decode_rm8(decoder, mode, rm);

            format!("mov {},{}", dst, src)
        }
        0xB0..=0xB7 => {
            let reg = opcode & 7;
            let value = decoder.read_u8();
            format!("mov {},{:02X}h", Register8::from(reg), value)
        }
        0x50..=0x57 => {
            let reg = opcode & 7;
            format!("push {}", Register16::from(reg))
        }
        0x58..=0x5F => {
            let reg = opcode & 7;
            format!("pop {}", Register16::from(reg))
        }
        0xB8..=0xBF => {
            let reg = opcode & 7;
            let value = decoder.read_u16();
            format!("mov {},{:04X}h", Register16::from(reg), value)
        }
        0x90 => "nop".into(),
        0xC3 => "ret".into(),
        _ => format!("db {:02X}h", opcode),
    };

    Instruction {
        address,
        size: (decoder.ip - address) as u8,
        text,
    }
}
