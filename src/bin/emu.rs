use std::io::{Read, Write};

use loader::{
    decoder::{Decoder, decode},
    emulator::{Cpu, Machine, Memory, VideoCard},
};

fn main() {
    let screen = VideoCard::new(80, 25);
    let memory = Memory::new();
    let mut machine = Machine::new(memory, screen);
    let mut cpu = Cpu::new();
    let args: Vec<String> = std::env::args().collect();
    let Some(filename) = args.get(1) else {
        eprintln!("Filename must be given");
        return;
    };
    let mut file = std::fs::File::open(filename).expect("Failed to open file");
    let mut content = Vec::new();
    file.read_to_end(&mut content).expect("File read correctly");
    machine.memory.load_com(&content);

    eprintln!("---- DawEmu86 ----");
    loop {
        let mut decoder = Decoder {
            cpu: &mut cpu,
            machine: &mut machine,
        };
        let instruction = decode(&mut decoder);
        let continue_process = cpu.execute(&mut machine, instruction.op);

        if machine.screen.dirty {
            print!("\x1b[H");
            print!("\x1b[?25l"); // schowaj kursor
            print!("{}", machine.screen);
            std::io::stdout().flush().unwrap();
            machine.screen.clean_dirty_flag();
        }

        if !continue_process {
            break;
        }
    }
    print!("\x1b[?25h"); // schowaj kursor
}
