use std::io::{Read, Write};

use loader::{
    decoder::{Decoder, decode},
    emulator::{Cpu, Screen},
    isa::{Op, Register8, Register16},
};

fn main() {
    let screen = Screen::new(80, 25);
    let mut cpu = Cpu::new(screen);
    let mut file = std::fs::File::open("dos/FDBANNER.COM").expect("Failed to open file");
    file.read(&mut cpu.memory[0x100..])
        .expect("Failed to read file");

    eprintln!("---- DawEmu86 ----");
    loop {
        let mut decoder = Decoder { cpu: &mut cpu };
        let ins = decode(&mut decoder);

        match ins.op {
            Op::Nop => {}
            Op::Mov { src, dst } => {
                let value = cpu.get_operand_value(&src);
                cpu.set_operand_value(&dst, value);
            }
            Op::Xor { src, dst } => {
                let v1 = cpu.get_operand_value(&src);
                let v2 = cpu.get_operand_value(&dst);
                let r = v1 ^ v2;
                cpu.flags.zero = r == 0;
                cpu.set_operand_value(&dst, r);
            }
            Op::PushCs => {
                let curret_sp = cpu.registers.read16(Register16::Sp);
                let new_sp = curret_sp.wrapping_sub(2);
                cpu.registers.write16(Register16::Sp, new_sp);
                let cs_val = 0u16;
                cpu.write_u16(new_sp, cs_val);
            }
            Op::PopDs => {
                let curret_sp = cpu.registers.read16(Register16::Sp);
                let _val = cpu.read_u16(curret_sp);
                cpu.registers
                    .write16(Register16::Sp, curret_sp.wrapping_add(2));
            }
            Op::Cld => {
                cpu.flags.direction = false;
            }
            Op::Lodsw => {
                let si_address = cpu.registers.read16(Register16::Si);
                let val = cpu.read_u16(si_address);
                cpu.registers.write16(Register16::Ax, val);
                cpu.registers.write16(
                    Register16::Si,
                    if cpu.flags.direction {
                        si_address.wrapping_sub(2)
                    } else {
                        si_address.wrapping_add(2)
                    },
                );
            }
            Op::Xchg { src, dst } => {
                let v1 = cpu.get_operand_value(&src);
                let v2 = cpu.get_operand_value(&dst);
                cpu.set_operand_value(&src, v2);
                cpu.set_operand_value(&dst, v1);
            }
            Op::PushReg16(reg) => {
                let curret_sp = cpu.registers.read16(Register16::Sp);
                let new_sp = curret_sp.wrapping_sub(2);
                cpu.registers.write16(Register16::Sp, new_sp);
                let val = cpu.registers.read16(reg);
                cpu.write_u16(new_sp, val);
            }
            Op::PopReg16(reg) => {
                let curret_sp = cpu.registers.read16(Register16::Sp);
                let value = cpu.read_u16(curret_sp);
                cpu.registers
                    .write16(Register16::Sp, curret_sp.wrapping_add(2));
                cpu.registers.write16(reg, value);
            }
            Op::Int(0x10) => {
                let ah = cpu.registers.read8(Register8::Ah);
                match ah {
                    0x02 => {
                        let page = cpu.registers.read8(Register8::Bh);
                        let row = cpu.registers.read8(Register8::Dh);
                        let col = cpu.registers.read8(Register8::Dl);
                        cpu.screen.set_cursor_pos(page, row, col);
                    }
                    0x03 => {
                        cpu.registers.write8(Register8::Ch, 0);
                        cpu.registers.write8(Register8::Cl, 15);
                        cpu.registers.write8(Register8::Dh, cpu.screen.current_row);
                        cpu.registers.write8(Register8::Dl, cpu.screen.current_col);
                    }
                    0x09 => {
                        let character = cpu.registers.read8(Register8::Al);
                        let page = cpu.registers.read8(Register8::Bh);
                        let attribute = cpu.registers.read8(Register8::Bl);
                        let count = cpu.registers.read16(Register16::Cx);

                        cpu.screen
                            .write_char_and_attr_at_current(page, character, attribute, count);
                    }
                    _ => {
                        panic!("Unhandled 0x10:{:02X}", ah)
                    }
                }
            }
            Op::Int(0x21) => {
                let ah = cpu.registers.read8(Register8::Ah);
                match ah {
                    0x02 => {
                        let data = cpu.registers.read8(Register8::Dl);
                        cpu.screen.write_char(data);
                    }
                    0x4C => {
                        let _exit_status = cpu.registers.read8(Register8::Al);
                        break;
                    }
                    _ => {
                        panic!("Unhandled 0x21:{:02X}", ah)
                    }
                }
            }
            Op::Cmp { src, dst } => {
                let src_val = cpu.get_operand_value(&src);
                let dst_val = cpu.get_operand_value(&dst);

                let (res, _overflow) = dst_val.overflowing_sub(src_val);
                cpu.flags.zero = res == 0;
            }
            Op::Test { src, dst } => {
                let src_val = cpu.get_operand_value(&src);
                let dst_val = cpu.get_operand_value(&dst);

                let res = dst_val & src_val;
                cpu.flags.zero = res == 0;
            }
            Op::Jnz(target) => {
                let dest = cpu.get_operand_value(&target);
                if !cpu.flags.zero {
                    cpu.ip = dest;
                }
            }
            Op::Jz(target) => {
                let dest = cpu.get_operand_value(&target);
                if cpu.flags.zero {
                    cpu.ip = dest;
                }
            }
            Op::Jmp(target) => {
                let dest = cpu.get_operand_value(&target);
                cpu.ip = dest;
            }
            Op::Dec(operand) => {
                let v = cpu.get_operand_value(&operand);
                cpu.set_operand_value(&operand, v.wrapping_sub(1));
            }
            Op::Inc(operand) => {
                let v = cpu.get_operand_value(&operand);
                cpu.set_operand_value(&operand, v.wrapping_add(1));
            }
            Op::Shl(operand) => {
                let v = cpu.get_operand_value(&operand);
                cpu.set_operand_value(&operand, v.wrapping_shl(1));
            }
            instruction => {
                println!("[ERROR] Unknown instruction: {instruction}.");
                break;
            }
        }

        if cpu.screen.dirty {
            print!("\x1b[H");
            print!("\x1b[?25l"); // schowaj kursor
            print!("{}", cpu.screen);
            std::io::stdout().flush().unwrap();
            cpu.screen.clean_dirty_flag();
        }
    }
    print!("\x1b[?25h"); // schowaj kursor
}
