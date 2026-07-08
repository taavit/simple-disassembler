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
        println!("{:04X}: {}", ins.address, ins.op);
    }
}
struct Instruction {
    address: u16,
    size: u8,
    op: Op,
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
    None,
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
            Self::None => write!(f, ""),
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

pub struct MemSpec {
    pub base: EffectiveAddressBase,
    /// Displacement
    pub disp: i16,
    pub is_direct: bool,
}

impl Into<Operand> for Register8 {
    fn into(self) -> Operand {
        Operand::Register8(self)
    }
}

pub enum Operand {
    Register8(Register8),
    Register16(Register16),
    Imm8(u8),
    Imm16(u16),
    RelAddress(u16),
    Mem8(MemSpec),
    Mem16(MemSpec),
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Register8(r) => r.fmt(f),
            Operand::Register16(r) => r.fmt(f),
            Operand::Imm16(v) => write!(f, "{:04X}h", v),
            Operand::Imm8(v) => write!(f, "{:02X}h", v),
            Operand::RelAddress(r) => write!(f, "label_{:04X}", r),
            Operand::Mem8(m) => {
                if m.is_direct {
                    write!(f, "byte ptr [0x{:04X}]", m.disp)
                } else if m.disp == 0 {
                    write!(f, "byte ptr [{}]", m.base)
                } else {
                    write!(f, "byte ptr [{}{:+05X}]", m.base, m.disp)
                }
            }
            Operand::Mem16(m) => {
                if m.is_direct {
                    write!(f, "word ptr [0x{:04X}]", m.disp)
                } else if m.disp == 0 {
                    write!(f, "word ptr [{}]", m.base)
                } else {
                    write!(f, "word ptr [{}{:+X}]", m.base, m.disp)
                }
            }
        }
    }
}

pub enum Op {
    Nop,
    Ret,
    Lodsw,

    PushEs,
    PopEs,
    PushCs,
    PopDs,
    Cld,

    Int(u8),
    PopReg16(Register16),
    PushReg16(Register16),
    Inc(Operand),
    Dec(Operand),
    Jmp(Operand),
    Jz(Operand),
    Jnz(Operand),

    Db(u8),

    Mov { src: Operand, dst: Operand },
    Xor { src: Operand, dst: Operand },
    Test { src: Operand, dst: Operand },
    Xchg { src: Operand, dst: Operand },

    Rol(Operand),
    Ror(Operand),
    Rcl(Operand),
    Rcr(Operand),
    Shl(Operand),
    Shr(Operand),
    Sar(Operand),

    Add { src: Operand, dst: Operand },
    Or { src: Operand, dst: Operand },
    Adc { src: Operand, dst: Operand },
    Sbb { src: Operand, dst: Operand },
    And { src: Operand, dst: Operand },
    Sub { src: Operand, dst: Operand },
    Cmp { src: Operand, dst: Operand },

    Invalid,
}

impl Display for Op {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Op::Nop => f.write_str("nop"),
            Op::Ret => f.write_str("ret"),
            Op::Lodsw => f.write_str("lodsw"),
            Op::PushEs => f.write_str("push es"),
            Op::PopEs => f.write_str("pop es"),
            Op::PushCs => f.write_str("push cs"),
            Op::PopDs => f.write_str("pop ds"),
            Op::Cld => f.write_str("cld"),
            Op::Int(num) => write!(f, "int {:02X}h", num),
            Op::PopReg16(reg) => write!(f, "pop {reg}"),
            Op::PushReg16(reg) => write!(f, "push {reg}"),
            Op::Inc(reg) => write!(f, "inc {reg}"),
            Op::Dec(reg) => write!(f, "dec {reg}"),

            Op::Jmp(j) => write!(f, "jmp {j}"),
            Op::Jz(j) => write!(f, "jz {j}"),
            Op::Jnz(j) => write!(f, "jnz {j}"),
            Op::Mov { src, dst } => write!(f, "mov {dst},{src}"),
            Op::Xor { src, dst } => write!(f, "xor {dst},{src}"),
            Op::Test { src, dst } => write!(f, "test {dst},{src}"),
            Op::Xchg { src, dst } => write!(f, "xchg {dst},{src}"),

            Op::Rol(o) => write!(f, "rol {},1", o),
            Op::Ror(o) => write!(f, "ror {},1", o),
            Op::Rcl(o) => write!(f, "rcl {},1", o),
            Op::Rcr(o) => write!(f, "rcr {},1", o),
            Op::Shl(o) => write!(f, "shl {},1", o),
            Op::Shr(o) => write!(f, "shr {},1", o),
            Op::Sar(o) => write!(f, "sar {},1", o),

            Op::Add { src, dst } => write!(f, "add {dst},{src}"),
            Op::Or { src, dst } => write!(f, "or {dst},{src}"),
            Op::Adc { src, dst } => write!(f, "adc {dst},{src}"),
            Op::Sbb { src, dst } => write!(f, "sbb {dst},{src}"),
            Op::And { src, dst } => write!(f, "and {dst},{src}"),
            Op::Sub { src, dst } => write!(f, "sub {dst},{src}"),
            Op::Cmp { src, dst } => write!(f, "cmp {dst},{src}"),

            Op::Db(v) => write!(f, "db {:02X}h", v),
            Op::Invalid => write!(f, "(invalid)"),
        }
    }
}

fn decode_rm8(decoder: &mut Decoder, mode: u8, rm: u8) -> Operand {
    match (mode, rm) {
        (0b00, 6) => {
            let addr = decoder.read_u16();
            Operand::Mem8(MemSpec {
                base: EffectiveAddressBase::None,
                disp: addr as i16,
                is_direct: true,
            })
        }
        (0b00, _) => Operand::Mem8(MemSpec {
            base: EffectiveAddressBase::from(rm),
            disp: 0,
            is_direct: false,
        }),
        (0b01, _) => {
            let disp = decoder.read_u8() as i8;
            Operand::Mem8(MemSpec {
                base: EffectiveAddressBase::from(rm),
                disp: disp as i16,
                is_direct: false,
            })
        }
        (0b10, _) => {
            let disp = decoder.read_u16() as i16;
            Operand::Mem8(MemSpec {
                base: EffectiveAddressBase::from(rm),
                disp: disp,
                is_direct: false,
            })
        }
        (0b11, _) => Operand::Register8(Register8::from(rm)),
        _ => unreachable!(),
    }
}

fn decode_rm16(decoder: &mut Decoder, mode: u8, rm: u8) -> Operand {
    match mode {
        0b11 => Operand::Register16(Register16::from(rm)),
        _ => {
            if let Operand::Mem8(m) = decode_rm8(decoder, mode, rm) {
                Operand::Mem16(m)
            } else {
                unreachable!()
            }
        }
    }
}

fn decode(decoder: &mut Decoder) -> Instruction {
    let address = decoder.ip;
    let opcode = decoder.read_u8();
    let op: Op = match opcode {
        0x06 => Op::PushEs,
        0x07 => Op::PopEs,
        0x0E => Op::PushCs,
        0xE9 => Op::Jmp(Operand::RelAddress(decoder.read_rel16())),
        0xAD => Op::Lodsw,
        0x1F => Op::PopDs,
        0xFC => Op::Cld,
        0xD0 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = decode_rm8(decoder, mode, rm);
            match reg {
                0 => Op::Rol(dst),
                1 => Op::Ror(dst),
                2 => Op::Rcl(dst),
                3 => Op::Rcr(dst),
                4 => Op::Shl(dst),
                5 => Op::Shr(dst),
                6 => Op::Invalid,
                7 => Op::Sar(dst),
                _ => unreachable!(),
            }
        }
        0x30 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let src = Register8::from(reg);
            let dst = decode_rm8(decoder, mode, rm);
            Op::Xor {
                src: Operand::Register8(src),
                dst,
            }
        }

        0x31 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let src = Register16::from(reg);
            let dst = decode_rm16(decoder, mode, rm);

            Op::Xor {
                src: Operand::Register16(src),
                dst,
            }
        }
        0x88 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let src = Register8::from(reg);
            let dst = decode_rm8(decoder, mode, rm);

            Op::Mov {
                src: Operand::Register8(src),
                dst,
            }
        }
        0x40..=0x47 => Op::Inc(Operand::Register16(Register16::from(opcode))),
        0xFE => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let operand = decode_rm8(decoder, mode, rm);
            match reg {
                0 => Op::Inc(operand),
                1 => Op::Dec(operand),
                _ => Op::Db(0xFE),
            }
        }
        0x80 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = decode_rm8(decoder, mode, rm);
            let imm = decoder.read_u8();
            let src = Operand::Imm8(imm);

            match reg {
                0 => Op::Add { dst, src },
                1 => Op::Or { dst, src },
                2 => Op::Adc { dst, src },
                3 => Op::Sbb { dst, src },
                4 => Op::And { dst, src },
                5 => Op::Sub { dst, src },
                6 => Op::Xor { dst, src },
                7 => Op::Cmp { dst, src },
                _ => unreachable!(),
            }
        }
        0xCD => {
            let num = decoder.read_u8();
            Op::Int(num)
        }
        0x84 => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let src = Register8::from(reg).into();
            let dst = decode_rm8(decoder, mode, rm);

            Op::Test { src, dst }
        }
        0x74 => Op::Jz(Operand::RelAddress(decoder.read_rel8())),
        0x75 => Op::Jnz(Operand::RelAddress(decoder.read_rel8())),
        0xEB => Op::Jmp(Operand::RelAddress(decoder.read_rel8())),
        0x86 => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let src = Register8::from(reg).into();
            let dst = decode_rm8(decoder, mode, rm);

            Op::Xchg { src, dst }
        }
        0x8A => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let dst = Operand::Register8(Register8::from(reg));
            let src = decode_rm8(decoder, mode, rm);
            Op::Mov { src, dst }
        }
        0xB0..=0xB7 => {
            let value = decoder.read_u8();
            Op::Mov {
                src: Operand::Imm8(value),
                dst: Operand::Register8(Register8::from(opcode)),
            }
        }
        0x50..=0x57 => Op::PushReg16(Register16::from(opcode)),
        0x58..=0x5F => Op::PopReg16(Register16::from(opcode)),
        0xB8..=0xBF => {
            let value = decoder.read_u16();
            Op::Mov {
                src: Operand::Imm16(value),
                dst: Operand::Register16(Register16::from(opcode)),
            }
        }
        0x90 => Op::Nop,
        0xC3 => Op::Ret,
        _ => Op::Db(opcode),
    };

    Instruction {
        address,
        size: (decoder.ip - address) as u8,
        op,
    }
}
