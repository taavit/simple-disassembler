use crate::{
    emulator::{Cpu, Machine},
    isa::{
        EffectiveAddressBase, MemSpec, Op,
        Operand::{self, Imm8},
        Register8, Register16, SegmentRegister,
    },
};

pub struct Decoder<'a> {
    pub cpu: &'a mut Cpu,
    pub machine: &'a mut Machine,
}

impl<'a> Decoder<'a> {
    pub fn read_u16(&mut self) -> u16 {
        let r = self
            .cpu
            .read_u16(self.machine, SegmentRegister::Cs, self.cpu.registers.ip());
        self.cpu.registers.step_ip_by(2);

        r
    }

    pub fn read_u8(&mut self) -> u8 {
        let r = self
            .cpu
            .read_u8(self.machine, SegmentRegister::Cs, self.cpu.registers.ip());
        self.cpu.registers.step_ip();

        r
    }

    pub fn read_rel8(&mut self) -> i16 {
        i8::from_ne_bytes([self.read_u8()]) as i16
    }

    pub fn read_rel16(&mut self) -> i16 {
        self.read_u16() as i16
    }
}

fn decode_modrm(byte: u8) -> (u8, u8, u8) {
    let mode = (byte >> 6) & 3;
    let reg = (byte >> 3) & 7;
    let rm = byte & 7;

    (mode, reg, rm)
}

fn decode_rm8(decoder: &mut Decoder, mode: u8, rm: u8) -> Operand {
    match (mode, rm) {
        (0b00, 6) => {
            let addr = decoder.read_u16();
            Operand::Mem8(MemSpec {
                base: EffectiveAddressBase::None,
                disp: addr as i16,
                is_direct: true,
                segment: None,
            })
        }
        (0b00, _) => Operand::Mem8(MemSpec {
            base: EffectiveAddressBase::from(rm),
            disp: 0,
            is_direct: false,
            segment: None,
        }),
        (0b01, _) => {
            let disp = decoder.read_u8() as i8;
            Operand::Mem8(MemSpec {
                base: EffectiveAddressBase::from(rm),
                disp: disp as i16,
                is_direct: false,
                segment: None,
            })
        }
        (0b10, _) => {
            let disp = decoder.read_u16() as i16;
            Operand::Mem8(MemSpec {
                base: EffectiveAddressBase::from(rm),
                is_direct: false,
                disp,
                segment: None,
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

pub fn decode(decoder: &mut Decoder) -> Instruction {
    let mut opcode = decoder.read_u8();
    let segment_override = match opcode {
        0x26 => Some(SegmentRegister::Es),
        0x2E => Some(SegmentRegister::Cs),
        0x36 => Some(SegmentRegister::Ss),
        0x3E => Some(SegmentRegister::Ds),
        _ => None,
    };
    if segment_override.is_some() {
        opcode = decoder.read_u8();
    }
    let address = decoder.cpu.registers.ip();
    let op: Op = match opcode {
        0x06 => Op::PushEs,
        0x07 => Op::PopEs,
        0x0E => Op::PushCs,
        0xE9 => Op::Jmp(Operand::RelAddress(decoder.read_rel16())),
        0xAD => Op::Lodsw,
        0x1F => Op::PopDs,
        0xF8 => Op::Clc,
        0xF9 => Op::Stc,
        0xFA => Op::Cli,
        0xFB => Op::Sti,
        0xFC => Op::Cld,
        0xFD => Op::Std,
        0xA1 => {
            let offset = decoder.read_u16();
            Op::Mov {
                dst: Operand::Register16(Register16::Ax),
                src: Operand::Mem16(MemSpec {
                    base: EffectiveAddressBase::None,
                    disp: offset as i16,
                    is_direct: true,
                    segment: Some(SegmentRegister::Ds),
                }),
            }
        }
        0xA3 => {
            let offset = decoder.read_u16();
            Op::Mov {
                src: Operand::Register16(Register16::Ax),
                dst: Operand::Mem16(MemSpec {
                    base: EffectiveAddressBase::None,
                    disp: offset as i16,
                    is_direct: true,
                    segment: Some(SegmentRegister::Ds),
                }),
            }
        }
        0x2D => {
            let imm = decoder.read_u16();
            Op::Sub {
                src: Operand::Imm16(imm),
                dst: Operand::Register16(Register16::Ax),
            }
        }
        0x05 => {
            let imm = decoder.read_u16();
            Op::Add {
                src: Operand::Imm16(imm),
                dst: Operand::Register16(Register16::Ax),
            }
        }
        0x3C => {
            let imm = decoder.read_u8();
            Op::Cmp {
                src: Operand::Register8(Register8::Al),
                dst: Operand::Imm16(imm as u16),
            }
        }
        0xD0 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = decode_rm8(decoder, mode, rm);
            let src = Imm8(1);
            match reg {
                0 => Op::Rol { dst, src },
                1 => Op::Ror { dst, src },
                2 => Op::Rcl { dst, src },
                3 => Op::Rcr { dst, src },
                4 => Op::Shl { dst, src },
                5 => Op::Shr { dst, src },
                6 => Op::Invalid,
                7 => Op::Sar { dst, src },
                _ => unreachable!(),
            }
        }
        0xD3 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = decode_rm16(decoder, mode, rm);
            let src = Operand::Register8(Register8::Cl);
            match reg {
                0 => Op::Rol { dst, src },
                1 => Op::Ror { dst, src },
                2 => Op::Rcl { dst, src },
                3 => Op::Rcr { dst, src },
                4 => Op::Shl { dst, src },
                5 => Op::Shr { dst, src },
                6 => Op::Invalid,
                7 => Op::Sar { dst, src },
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
        0x32 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = Register8::from(reg);
            let src = decode_rm8(decoder, mode, rm);
            Op::Xor {
                dst: Operand::Register8(dst),
                src,
            }
        }

        0x03 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = Register16::from(reg);
            let src = decode_rm16(decoder, mode, rm);
            Op::Add {
                src,
                dst: Operand::Register16(dst),
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
        0x33 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = Register16::from(reg);
            let src = decode_rm16(decoder, mode, rm);

            Op::Xor {
                dst: Operand::Register16(dst),
                src,
            }
        }
        0xC6 => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);
            if reg != 0 {
                Op::Invalid
            } else {
                let dst = decode_rm8(decoder, mode, rm);
                let src = Operand::Imm8(decoder.read_u8());

                Op::Mov { src, dst }
            }
        }
        0xC7 => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);
            if reg != 0 {
                Op::Invalid
            } else {
                let dst = decode_rm16(decoder, mode, rm);
                let src = Operand::Imm16(decoder.read_u16());

                Op::Mov { src, dst }
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
        0x89 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let src = Register16::from(reg);
            let dst = decode_rm16(decoder, mode, rm);

            Op::Mov {
                src: Operand::Register16(src),
                dst,
            }
        }
        0x8B => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let dst = Operand::Register16(Register16::from(reg));
            let src = decode_rm16(decoder, mode, rm);

            Op::Mov { src, dst }
        }
        0x8C => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let src = Operand::SegmentRegister(SegmentRegister::from(reg));
            let dst = decode_rm16(decoder, mode, rm);

            Op::Mov { src, dst }
        }
        0x8E => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let src = decode_rm16(decoder, mode, rm);
            let dst = Operand::SegmentRegister(SegmentRegister::from(reg));

            Op::Mov { src, dst }
        }
        0x83 => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);

            let dst = decode_rm16(decoder, mode, rm);
            let imm = decoder.read_u8() as i8 as i16 as u16;
            let src = Operand::Imm16(imm);

            match reg {
                0 => Op::Add { src, dst },
                1 => Op::Or { src, dst },
                2 => Op::Adc { src, dst },
                3 => Op::Sbb { src, dst },
                4 => Op::And { src, dst },
                5 => Op::Sub { src, dst },
                6 => Op::Xor { src, dst },
                7 => Op::Cmp { src, dst },
                _ => unreachable!(),
            }
        }
        0x3D => {
            let imm = decoder.read_u16();
            Op::Cmp {
                src: Operand::Imm16(imm),
                dst: Operand::Register16(Register16::Ax),
            }
        }
        0x40..=0x47 => Op::Inc(Operand::Register16(Register16::from(opcode))),
        0x48..=0x4F => Op::Dec(Operand::Register16(Register16::from(opcode))),
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
        0x81 => {
            let moderm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(moderm);
            let dst = decode_rm16(decoder, mode, rm);
            let imm = decoder.read_u16();
            let src = Operand::Imm16(imm);

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
        0xE3 => Op::Jcxz(Operand::RelAddress(decoder.read_rel8())),
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
        0xE8 => {
            let rel = decoder.read_rel16();
            Op::Call {
                target: Operand::RelAddress(rel),
            }
        }
        0x7F => Op::Jg(Operand::RelAddress(decoder.read_rel8())),
        0xD1 => {
            let modrm = decoder.read_u8();
            let (mode, reg, rm) = decode_modrm(modrm);
            let dst = decode_rm16(decoder, mode, rm);
            let src = Imm8(1);

            match reg {
                0 => Op::Rol { dst, src },
                1 => Op::Ror { dst, src },
                2 => Op::Rcl { dst, src },
                3 => Op::Rcr { dst, src },
                4 => Op::Shl { dst, src },
                5 => Op::Shr { dst, src },
                6 => Op::Invalid,
                7 => Op::Sar { dst, src },
                _ => unreachable!(),
            }
        }
        0x90 => Op::Nop,
        0xC3 => Op::Ret,
        _ => Op::Db(opcode),
    };

    Instruction {
        segment_override,
        address,
        size: (decoder.cpu.registers.ip() - address) as u8,
        op,
    }
}

pub struct Instruction {
    pub segment_override: Option<SegmentRegister>,
    pub address: u16,
    pub size: u8,
    pub op: Op,
}
