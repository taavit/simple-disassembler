#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
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

impl From<Register8> for Operand {
    fn from(val: Register8) -> Self {
        Operand::Register8(val)
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

pub enum Op {
    Nop,
    Ret,
    Lodsw,

    PushEs,
    PopEs,
    PushCs,
    PopDs,
    Cld,
    Std,

    Int(u8),
    PopReg16(Register16),
    PushReg16(Register16),
    Inc(Operand),
    Dec(Operand),
    Jmp(Operand),
    Jz(Operand),
    Jnz(Operand),
    Call { target: Operand },

    Db(u8),

    Mov { src: Operand, dst: Operand },
    Xor { src: Operand, dst: Operand },
    Test { src: Operand, dst: Operand },
    Xchg { src: Operand, dst: Operand },

    Rol { src: Operand, dst: Operand },
    Ror { src: Operand, dst: Operand },
    Rcl { src: Operand, dst: Operand },
    Rcr { src: Operand, dst: Operand },
    Shl { src: Operand, dst: Operand },
    Shr { src: Operand, dst: Operand },
    Sar { src: Operand, dst: Operand },

    Add { src: Operand, dst: Operand },
    Or { src: Operand, dst: Operand },
    Adc { src: Operand, dst: Operand },
    Sbb { src: Operand, dst: Operand },
    And { src: Operand, dst: Operand },
    Sub { src: Operand, dst: Operand },
    Cmp { src: Operand, dst: Operand },

    Invalid,
}
