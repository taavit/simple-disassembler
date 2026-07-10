use core::fmt::Display;

use crate::isa::{EffectiveAddressBase, Op, Operand, Register8, Register16};

impl Display for Register16 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

impl Display for Register8 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

impl Display for EffectiveAddressBase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

impl Display for Operand {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Operand::Register8(r) => r.fmt(f),
            Operand::Register16(r) => r.fmt(f),
            Operand::Imm16(v) => write!(f, "{:04X}h", v),
            Operand::Imm8(v) => write!(f, "{:02X}h", v),
            Operand::RelAddress(r) => write!(f, "{:04X}h", r),
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
