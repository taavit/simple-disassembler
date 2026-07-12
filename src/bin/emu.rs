use std::io::{Read, Write};

use loader::{
    decoder::{Decoder, decode},
    emulator::{Cpu, VideoCard},
};

fn main() {
    let screen = VideoCard::new(80, 25);
    let mut cpu = Cpu::new(screen);
    let args: Vec<String> = std::env::args().collect();
    // let mut file = std::fs::File::open("win/WINDOWS/WIN.COM").expect("Failed to open file");
    let Some(filename) = args.get(1) else {
        eprintln!("Filename must be given");
        return;
    };
    let mut file = std::fs::File::open(filename).expect("Failed to open file");
    file.read(&mut cpu.memory[0x100..])
        .expect("Failed to read file");

    eprintln!("---- DawEmu86 ----");
    loop {
        let mut decoder = Decoder { cpu: &mut cpu };
        let instruction = decode(&mut decoder);
        let continue_process = cpu.execute(instruction.op);

        if cpu.screen.dirty {
            print!("\x1b[H");
            print!("\x1b[?25l"); // schowaj kursor
            print!("{}", cpu.screen);
            std::io::stdout().flush().unwrap();
            cpu.screen.clean_dirty_flag();
        }

        if !continue_process {
            break;
        }
    }
    print!("\x1b[?25h"); // schowaj kursor
}
