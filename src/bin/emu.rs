use std::io::Read;

use loader::{
    decoder::{Decoder, decode}, emulator::{Cpu, Screen}, isa::{Op, Register8, Register16},
};

fn cp437_to_unicode(byte: u8, screen: &mut Screen) -> char {
    match byte {
        0x0D => {
            screen.cursor_col = 0;
            '\r'
        }, // CR
        0x0A => {
            screen.cursor_row = screen.cursor_row.wrapping_add(1);
            '\n' // LF
        }
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

fn get_ansi_color(dos_color: u8) -> &'static str {
    // Interesują nas 4 dolne bity (0-15) dla koloru tekstu
    match dos_color & 0x0F {
        0x00 => "\x1b[30m", // Czarny
        0x01 => "\x1b[34m", // Niebieski
        0x02 => "\x1b[32m", // Zielony (często używany do pasków postępu!)
        0x03 => "\x1b[36m", // Cyjan
        0x04 => "\x1b[31m", // Czerwony
        0x05 => "\x1b[35m", // Magenta
        0x06 => "\x1b[33m", // Brązowy / Żółty
        0x07 => "\x1b[37m", // Jasnoszary (standardowy)
        0x08 => "\x1b[90m", // Ciemnoszary
        0x09 => "\x1b[94m", // Jasnoniebieski
        0x0A => "\x1b[92m", // Jasnozielony
        0x0B => "\x1b[96m", // Jasny cyjan
        0x0C => "\x1b[91m", // Jasnoczerwony
        0x0D => "\x1b[95m", // Jasna magenta
        0x0E => "\x1b[93m", // Jasnożółty
        0x0F => "\x1b[97m", // Biały
        _ => "\x1b[0m",
    }
}

fn main() {
    let mut cpu = Cpu::new();
    let mut file = std::fs::File::open("dos/FDBANNER.COM").expect("Failed to open file");
    file.read(&mut cpu.memory[0x100..])
        .expect("Failed to read file");

    eprintln!("---- DawEmu86 ----");
    print!("\x1b[2J\x1b[3J\x1b[H"); // wyczyść ekran + scrollback
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
                        // Set Cursor
                        cpu.screen.cursor_row = cpu.registers.read8(Register8::Dh);
                        cpu.screen.cursor_col = cpu.registers.read8(Register8::Dl);
                        print!(
                            "\x1b[{};{}H",
                            cpu.screen.cursor_row + 1,
                            cpu.screen.cursor_col + 1
                        );
                    }
                    0x03 => {
                        // Get Cursor
                        cpu.registers.write8(Register8::Dh, cpu.screen.cursor_row);
                        cpu.registers.write8(Register8::Dl, cpu.screen.cursor_col);
                        cpu.registers.write8(Register8::Ch, 0);
                        cpu.registers.write8(Register8::Cl, 7);
                    }
                    0x09 => {
                        // Write Char + Attr
                        let ch = cpu.registers.read8(Register8::Al);
                        let attr = cpu.registers.read8(Register8::Bl);
                        let count = cpu.registers.read16(Register16::Cx);

                        print!("{}", get_ansi_color(attr));
                        for _ in 0..count {
                            print!("{}", cp437_to_unicode(ch, &mut cpu.screen));
                            cpu.screen.cursor_col = cpu.screen.cursor_col.wrapping_add(1);
                        }
                        print!("\x1b[0m");
                    }
                    _ => {}
                }
            }
            Op::Int(0x21) => {
                let ah = cpu.registers.read8(Register8::Ah);
                match ah {
                    0x02 => {
                        let dl = cpu.registers.read8(Register8::Dl);
                        print!("{}", cp437_to_unicode(dl, &mut cpu.screen));
                        cpu.registers.write8(Register8::Al, dl);
                        dbg!(&cpu.screen, dl);
                    }
                    0x09 => {
                        let mut addr = cpu.registers.read16(Register16::Dx);
                        loop {
                            let byte = cpu.read_u8(addr);
                            if byte == b'$' {
                                break;
                            }
                            let ch = cp437_to_unicode(byte, &mut cpu.screen);
                            print!("{}", ch);
                            addr = addr.wrapping_add(1);
                        }
                    }
                    0x4c => {
                        let status = cpu.registers.read8(Register8::Al);
                        println!("[END] Program quit with code: {status}");
                        break;
                    }
                    _ => {
                        println!("[ERROR] Unknown interrupt param {:02X}", ah);
                        break;
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
    }
}
