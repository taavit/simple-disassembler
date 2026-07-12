use std::io::Read;

use loader::{
    decoder::{Decoder, decode},
    emulator::{Cpu, Screen},
};

fn main() {
    let mut file = std::fs::File::open("dos/FDBANNER.COM").expect("Failed to open file");
    let mut cpu = Cpu::new(Screen::new(80, 25));
    file.read(&mut cpu.memory[0x100..])
        .expect("Failed to read file");

    let decoder = Decoder { cpu: &mut cpu };
}
