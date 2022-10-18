use bracket_lib::prelude::*;
use rand::prelude::*;
use std::{fs::File, io::Read};

const CHIP8_HEIGHT: usize = 32;
const CHIP8_WIDTH: usize = 64;
const CHIP8_RAMSIZE: usize = 4096;
const ROM_FILENAME: &str = "Maze [David Winter, 199x].ch8";

struct Chip8System {
    ram: [u8; CHIP8_RAMSIZE],
    vram: [[bool; CHIP8_HEIGHT]; CHIP8_WIDTH],
    v: [u8; 16],
    i: usize,
    stack: [usize; 16],
    sp: u8,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [bool; 16],
    pc: usize,
}

impl Chip8System {
    fn new() -> Chip8System {
        println!("Chip8System::new()");
        Chip8System {
            ram: [0; CHIP8_RAMSIZE],
            vram: [[false; CHIP8_HEIGHT]; CHIP8_WIDTH],
            v: [0; 16],
            i: 0,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [false; 16],
            pc: 0x200,
        }
    }

    fn execute_opcode(&mut self) {
        let opcode: u16 = (self.ram[self.pc] as u16) << 8 | self.ram[self.pc + 1] as u16;

        let op = (
            ((opcode & 0xF000) >> 12) as u8,
            ((opcode & 0x0F00) >> 8) as u8,
            ((opcode & 0x00F0) >> 4) as u8,
            (opcode & 0x000F) as u8,
        );
        println!("op={:x?}", op);

        match op {
            //Clear screen
            (_, _, 0x0E, 0x00) => {
                self.vram = [[false; CHIP8_HEIGHT]; CHIP8_WIDTH];
            }
            //Return from subroutine
            (_, _, 0x0E, 0x0E) => {
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize];
            }
            //Jump to address NNN
            (0x01, n1, n2, n3) => {
                self.pc = ((n1 as usize) << 8) + ((n2 as usize) << 4) + n3 as usize;
            }
            //Call subroutine at NNN,
            (0x02, n1, n2, n3) => {
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = ((n1 as usize) << 8) + ((n2 as usize) << 4) + n3 as usize;
            }
            //Skip next instruction if VX equals NN
            (0x03, vx, n1, n2) => {
                if self.v[vx as usize] == (n1 << 4) + n2 {
                    self.pc += 2;
                }
            }
            //Skip next instruction if VX does NOT equal NN
            (0x04, vx, n1, n2) => {
                if self.v[vx as usize] != (n1 << 4) + n2 {
                    self.pc += 2;
                }
            }
            //Skip the next instruction if VX equals VY
            (0x05, vx, vy, _) => {
                if self.v[vx as usize] == self.v[vy as usize] {
                    self.pc += 2;
                }
            }
            //Set VX to NN
            (0x06, vx, n1, n2) => {
                self.v[vx as usize] = (n1 << 4) + n2;
            }
            //Add NN to VX (dont change carry flag)
            (0x07, vx, n1, n2) => {
                self.v[vx as usize] = self.v[vx as usize].wrapping_add((n1 << 4) + n2);
            }
            //Set VX to value of VY
            (0x08, vx, vy, 0x00) => {
                self.v[vx as usize] = self.v[vy as usize];
            }
            //Set VX to (VX OR VY)
            (0x08, vx, vy, 0x01) => {
                self.v[vx as usize] = self.v[vx as usize] | self.v[vy as usize];
            }
            //Set VX to (VX AND VY)
            (0x08, vx, vy, 0x02) => {
                self.v[vx as usize] = self.v[vx as usize] & self.v[vy as usize];
            }
            //Set VX to (VX XOR VY)
            (0x08, vx, vy, 0x03) => {
                self.v[vx as usize] = self.v[vx as usize] ^ self.v[vy as usize];
            }
            //Add VY to VX, set VF to 1 if carry, 0 if not
            (0x08, vx, vy, 0x04) => {
                let (result, overflow) = self.v[vx as usize].overflowing_add(self.v[vy as usize]);
                self.v[vx as usize] = result;
                self.v[0x0F] = overflow.into();
            }
            //Subtract VY from VX, set VF to 0 if borrow, 1 if not
            (0x08, vx, vy, 0x05) => {
                let (result, overflow) = self.v[vx as usize].overflowing_sub(self.v[vy as usize]);
                self.v[vx as usize] = result;
                self.v[0x0F] = (!overflow).into();
            }
            //Store least significant bit of VX in VF, shift VX right by 1
            (0x08, vx, _, 0x06) => {
                self.v[0x0F] = self.v[vx as usize] & 0b0000_0001;
                self.v[vx as usize] = self.v[vx as usize] >> 1;
            }
            //Set VX to (VY - VX), set VF to 0 if borrow, 1 if not
            (0x08, vx, vy, 0x07) => {
                let (result, overflow) = self.v[vy as usize].overflowing_sub(self.v[vx as usize]);
                self.v[vx as usize] = result;
                self.v[0x0F] = (!overflow).into();
            }
            //Store the most significant bit of VX in VF, shift VX left by 1
            (0x08, vx, _, 0x0E) => {
                self.v[0x0F] = self.v[vx as usize] & 0b1000_0000;
                self.v[vx as usize] = self.v[vx as usize] << 1;
            }
            //Skip the next instruction if VX does NOT equal VY
            (0x09, vx, vy, _) => {
                if self.v[vx as usize] != self.v[vy as usize] {
                    self.pc += 2;
                }
            }
            //Set I to NNN
            (0x0A, n1, n2, n3) => {
                self.i = ((n1 as usize) << 8) + ((n2 as usize) << 4) + n3 as usize;
            }
            //Jump to NNN plus V0
            (0x0B, n1, n2, n3) => {
                self.pc = (((n1 as usize) << 8) + ((n2 as usize) << 4) + n3 as usize)
                    + self.v[0x00] as usize;
            }
            //Set VX to (RandomNumber(0-255) AND NN)
            (0x0C, vx, n1, n2) => {
                self.v[vx as usize] = thread_rng().gen::<u8>() & ((n1 << 4) + n2);
            }
            //Draw sprite, refer to Wikipedia for description
            //stolen from https://github.com/starrhorne/chip8-rust/blob/345602a97288fd8d69dafd6684e8f51cd38e95e2/src/processor.rs
            //because WHAT THE FUCK
            (0x0D, vx, vy, n) => {
                for byte in 0..n {
                    let y = (self.v[vy as usize] as usize + byte as usize) % CHIP8_HEIGHT;
                    for bit in 0..8 {
                        let x = (self.v[vx as usize] as usize + bit as usize) % CHIP8_WIDTH;
                        let color = ((self.ram[self.i + byte as usize] >> (7 - bit)) & 1) != 0;
                        self.v[0x0F] |= (color & (self.vram[x][y])) as u8;
                        self.vram[x][y] ^= color;
                    }
                }
            }
            //Skip next instruction if key in VX is being pressed
            (0x0E, vx, 0x09, 0x0E) => {
                if self.keypad[self.v[vx as usize] as usize] {
                    self.pc += 2;
                }
            }
            //Skip next instruction if key in VX is NOT being pressed
            (0x0E, vx, 0x0A, 0x01) => {
                if !self.keypad[self.v[vx as usize] as usize] {
                    self.pc += 2;
                }
            }
            //Set VX to value of delay timer
            (0x0F, vx, 0x00, 0x07) => {
                self.v[vx as usize] = self.delay_timer;
            }
            //Pause until a key is pressed, store key in VX
            (0x0F, vx, 0x00, 0x0A) => {
                todo!()
            }
            //Set delay timer to VX
            (0x0F, vx, 0x01, 0x05) => {
                self.delay_timer = self.v[vx as usize];
            }
            //Set sound timer to VX
            (0x0F, vx, 0x01, 0x08) => {
                self.sound_timer = self.v[vx as usize];
            }
            //Add VX to I
            (0x0F, vx, 0x01, 0x0E) => {
                self.i += self.v[vx as usize] as usize;
            }
            //some font shit come back to this FX29
            (0x0F, vx, 0x02, 0x09) => {
                todo!()
            }
            //Store binary version of VX in I, I[0] = hundreds, I[1] = tens, I[2] = ones
            //thanks to https://github.com/starrhorne/chip8-rust/blob/345602a97288fd8d69dafd6684e8f51cd38e95e2/src/processor.rs#L408
            (0x0F, vx, 0x03, 0x03) => {
                self.ram[self.i] = self.v[vx as usize] / 100;
                self.ram[self.i + 1] = (self.v[vx as usize] % 100) / 10;
                self.ram[self.i + 2] = self.v[vx as usize] % 10;
            }
            //Store from V0 to VX starting at I
            (0x0F, vx, 0x05, 0x05) => {
                for index in 0..vx {
                    self.ram[self.i + index as usize] = self.v[index as usize];
                }
            }
            //Fill V0 to VX with memory starting from I
            (0x0F, vx, 0x06, 0x05) => {
                for index in 0..vx {
                    self.v[index as usize] = self.ram[self.i + index as usize];
                }
            }
            _ => todo!(), //invalid opcode
        }
        self.pc += 2;
    }
}

impl GameState for Chip8System {
    fn tick(&mut self, ctx: &mut BTerm) {
        self.execute_opcode();
        for y in 0..CHIP8_HEIGHT {
            for x in 0..CHIP8_WIDTH {
                ctx.set(
                    x,
                    y,
                    if self.vram[x][y] { YELLOW } else { BLACK },
                    BLACK,
                    to_cp437('█'),
                );
            }
        }
    }
}

fn main() -> BError {
    let mut system = Chip8System::new();

    let mut rom_file = File::open(format!("D:\\User\\Downloads\\chip8\\{}", ROM_FILENAME)).unwrap();
    rom_file
        .read(&mut system.ram[system.pc..])
        .expect("couldn't read rom into emulator");

    let context = BTermBuilder::simple(CHIP8_WIDTH, CHIP8_HEIGHT)?
        .with_title("CHIP8 EMULATOR")
        .build()?;

    main_loop(context, system)
}
