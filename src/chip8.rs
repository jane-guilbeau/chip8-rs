use rand::{rngs::SmallRng, SeedableRng, RngCore};

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0,
    0x20, 0x60, 0x20, 0x20, 0x70,
    0xF0, 0x10, 0xF0, 0x80, 0xF0,
    0xF0, 0x10, 0xF0, 0x10, 0xF0,
    0x90, 0x90, 0xF0, 0x10, 0x10,
    0xF0, 0x80, 0xF0, 0x10, 0xF0,
    0xF0, 0x80, 0xF0, 0x90, 0xF0,
    0xF0, 0x10, 0x20, 0x40, 0x40,
    0xF0, 0x90, 0xF0, 0x90, 0xF0,
    0xF0, 0x90, 0xF0, 0x10, 0xF0,
    0xF0, 0x90, 0xF0, 0x90, 0x90,
    0xE0, 0x90, 0xE0, 0x90, 0xE0,
    0xF0, 0x80, 0x80, 0x80, 0xF0,
    0xE0, 0x90, 0x90, 0x90, 0xE0,
    0xF0, 0x80, 0x80, 0x80, 0xF0,
    0xF0, 0x80, 0xF0, 0x80, 0x80
];

pub struct Chip8 {
    display: [[bool; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
    memory: [u8; 4096],
    stack: Vec<u16>,
    pc: u16,
    registers: [u8; 16],
    index_register: u16,
    delay_timer: u8,
    sound_timer: u8,
    rng: SmallRng,
    input: [bool; 16],
}

impl Chip8 {
    pub fn new() -> Chip8 {
        let mut chip8 = Chip8 {
            display: [[false; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
            memory: [0; 4096],
            stack: Vec::new(),
            pc: 0x200,
            registers: [0; 16],
            index_register: 0,
            delay_timer: 0,
            sound_timer: 0,
            rng: SmallRng::from_entropy(),
            input: [false; 16],
        };

        chip8.load_to_memory(&FONT, 0x050);

        //chip8.execute_instruction(0x6055);
        //chip8.execute_instruction(0x7001);

        chip8
    }
    
    // Is called for every CPU cycle, which varies depending
    // on settings
    pub fn update(&mut self) {
        let instruction = self.fetch_instruction();
        self.execute_instruction(instruction);
    }
    
    // Is called at a rate of approximately 60Hz
    pub fn draw(&mut self) {
        // Timers are decremented in draw phase because they
        // should decrement 60 times per second
        if self.delay_timer > 0 { self.delay_timer -= 1; }
        if self.sound_timer > 0 { self.sound_timer -= 1; }
    }
    
    pub fn load_to_memory(&mut self, data: &[u8], start_pos: usize) {
        for i in 0..data.len() {
            self.memory[start_pos+i] = data[i];
        }
    }

    pub fn get_display(&self) -> &[[bool; DISPLAY_WIDTH]; DISPLAY_HEIGHT] {
        &self.display
    }
    
    pub fn get_sound_timer(&self) -> u8 {
        self.sound_timer
    }

    fn get_pixel(&self, x: usize, y: usize) -> Option<bool> {
        if x < DISPLAY_WIDTH && y < DISPLAY_HEIGHT { Some(self.display[y][x]) }
        else { None }
    }

    fn set_pixel(&mut self, x: usize, y: usize, value: bool) {
        if x < DISPLAY_WIDTH && y < DISPLAY_HEIGHT { self.display[y][x] = value; }
    }

    pub fn set_key(&mut self, key: usize, value: bool) {
        self.input[key] = value;
    }

    pub fn fetch_instruction(&mut self) -> u16 {
        let b1 = self.memory[self.pc as usize] as u16;
        let b2 = self.memory[(self.pc+1) as usize] as u16;
        let instruction = (b1 << 8) + b2;

        self.pc += 2;

        instruction
    }

    pub fn execute_instruction(&mut self, instruction: u16) {
        // Split instruction into four half-bytes
        let n1 = ((instruction >> 12) & 0xf) as u8;
        let n2 = ((instruction >> 8) & 0xf) as u8;
        let n3 = ((instruction >> 4) & 0xf) as u8;
        let n4 = (instruction & 0xf) as u8;

        // Last two half-bytes of instruction
        let n3n4 = (instruction &0xff) as u8;

        // Last three half-bytes of instruction
        let n2n3n4 = (instruction &0xfff) as u16;
        
        // Match instruction to opcode
        match n1 {
            0x0 => {
                // 00E0: Clear screen
                if instruction == 0x00e0 {
                    self.clear_screen();
                }
                // 00EE: Return from subroutine
                else if instruction == 0x00ee {
                    self.pc = self.stack.pop().unwrap();
                }
            },
            // 1NNN: Jump
            0x1 => {
                // Decrement PC to cancel out increment during fetch stage
                //self.pc -= 2;

                self.pc = n2n3n4;
            },
            // 2NNN: Call subroutine
            0x2 => {
                
                // Decrement PC to cancel out increment during fetch stage
                //self.pc -= 2;
                
                self.stack.push(self.pc);

                self.pc = n2n3n4;
                
            },
            // 3XNN: Conditional (register equal to value)
            0x3 => {
                if self.registers[n2 as usize] == n3n4 {
                    self.pc += 2;
                }
            },
            // 4XNN: Conditional (register not equal to value)
            0x4 => {
                if self.registers[n2 as usize] != n3n4 {
                    self.pc += 2;
                }
            },
            // 5XY0: Conditional (registers equal)
            0x5 => {
                if self.registers[n2 as usize] == self.registers[n3 as usize] {
                    self.pc += 2;
                }
            },
            // 6XNN: Set
            0x6 => {
                self.registers[n2 as usize] = n3n4;
            },
            // 7XNN: Add
            0x7 => {
                // Convert to u16 to handle overflow then convert back to u8 and add
                let mut buffer: u16 = self.registers[n2 as usize] as u16;
                buffer += n3n4 as u16;
                self.registers[n2 as usize] = (buffer % 256) as u8;
            },
            0x8 => {
                match n4 {
                    // 8XY0: Set
                    0x0 => {
                        self.registers[n2 as usize] = self.registers[n3 as usize];
                    },
                    // 8XY1: Binary OR
                    0x1 => {
                        self.registers[n2 as usize] |= self.registers[n3 as usize];
                    },
                    // 8XY2: Binary AND
                    0x2 => {
                        self.registers[n2 as usize] &= self.registers[n3 as usize];
                    },
                    // 8XY3: Logical XOR
                    0x3 => {
                        self.registers[n2 as usize] ^= self.registers[n3 as usize];
                    },
                    // 8XY4: Add
                    0x4 => {
                        let mut buffer: u16 = self.registers[n2 as usize] as u16;
                        buffer += self.registers[n3 as usize] as u16;
                        self.registers[0xf] = if buffer > 255 { 1 } else { 0 };
                        self.registers[n2 as usize] = (buffer % 256) as u8;
                    },
                    // 8XY5: Subtract (VX - VY)
                    0x5 => {
                        if self.registers[n2 as usize] >= self.registers[n3 as usize] {
                            self.registers[n2 as usize] -= self.registers[n3 as usize];
                            self.registers[0xf] = 1;
                        } else {
                            self.registers[n2 as usize] = 255 - (self.registers[n3 as usize] - self.registers[n2 as usize]) + 1;
                            self.registers[0xf] = 0;
                        }
                    },
                    // 8XY6: Shift right
                    0x6 => {
                        //self.registers[n2 as usize] = self.registers[n3 as usize];
                        self.registers[0xf] = self.registers[n2 as usize] & 1;
                        self.registers[n2 as usize] >>= 1;
                    }
                    // 8XY7: Subtract (VY - VX)
                    0x7 => {
                        if self.registers[n3 as usize] >= self.registers[n2 as usize] {
                            self.registers[n3 as usize] -= self.registers[n2 as usize];
                            self.registers[0xf] = 1;
                        } else {
                            self.registers[n3 as usize] = 255 - (self.registers[n2 as usize] - self.registers[n3 as usize]) + 1;
                            self.registers[0xf] = 0;
                        }
                    },
                    // 8XYE: Shift left
                    0xe => {
                        //self.registers[n2 as usize] = self.registers[n3 as usize];
                        self.registers[0xf] = (self.registers[n2 as usize] >> 7) & 1;
                        self.registers[n2 as usize] <<= 1;
                    }
                    _ => {}
                }
            },
            // 9XY0: Conditional (registers not equal)
            0x9 => {
                if self.registers[n2 as usize] != self.registers[n3 as usize] {
                    self.pc += 2;
                }
            },
            // ANNN: Set index
            0xa => {
                self.index_register = n2n3n4;
            },
            // BNNN: Jump with offset
            0xb => {
                self.pc = n2n3n4 + self.registers[0] as u16;
            },
            // CXNN: Random
            0xc => {
                // Generate 4-bit random integer
                let random = (self.rng.next_u32() & 0b1111) as u8;

                self.registers[n2 as usize] = random & n3n4;
            },
            // DXYN: Display
            0xd => {
                let x = self.registers[n2 as usize] % DISPLAY_WIDTH as u8;
                let y = self.registers[n3 as usize] % DISPLAY_HEIGHT as u8;
                self.draw_sprite(x, y, n4);
            },
            0xe => {
                match n3n4 {
                    // EX9E: Skip if key
                    0x9e => {
                        let key = self.registers[n2 as usize];
                        if self.input[key as usize] {
                            self.pc += 2
                        }
                    },
                    // EXA1: Skip if not key
                    0xa1 => {
                        let key = self.registers[n2 as usize];
                        if !self.input[key as usize] {
                            self.pc += 2
                        }
                    },
                    _ => {},
                }
            },
            0xf => {
                match n3n4 {
                    // FX07: Set register to delay timer
                    0x07 => {
                        self.registers[n2 as usize] = self.delay_timer;
                    },
                    // FX15: Set delay timer to register
                    0x15 => {
                        self.delay_timer = self.registers[n2 as usize];
                    },
                    // FX18: Set sound timer to register
                    0x18 => {
                        self.sound_timer = self.registers[n2 as usize];
                    },
                    // FX1E: Add to index
                    0x1e => {
                        self.index_register += self.registers[n2 as usize] as u16;
                    },
                    // FX0A: Wait for key
                    0x0a => {
                        for key in self.input {
                            if key { break; }

                            self.pc -= 2;
                        }
                    },
                    // FX29: Font character
                    0x29 => {
                        // Desired hexadecimal character to locate in font
                        let char = self.registers[n2 as usize] & 0b1111;

                        self.index_register = self.memory[(0x050 + 5 * char) as usize] as u16;
                    },
                    // FX33: Binary-coded decimal conversion
                    0x33 => {
                        let num = self.registers[n2 as usize];

                        let index = self.index_register as usize;
                        self.memory[index] = (num / 100) % 10;
                        self.memory[index + 1] = (num / 10) % 10;
                        self.memory[index + 2] = num % 10;
                    },
                    // FX55: Store registers in memory
                    0x55 => {
                        for i in 0..n2+1 {
                            self.memory[(self.index_register + i as u16) as usize] = self.registers[i as usize];
                        }
                    },
                    // FX65: Load registers from memory
                    0x65 => {
                        for i in 0..n2+1 {
                            self.registers[i as usize] = self.memory[(self.index_register + i as u16) as usize];
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    fn clear_screen(&mut self) {
        for i in 0..DISPLAY_HEIGHT {
            for j in 0..DISPLAY_WIDTH {
                self.set_pixel(j, i, false);
            }
        }
    }

    fn draw_sprite(&mut self, x: u8, y: u8, height: u8) {
        self.registers[15] = 0;

        let mut index = self.index_register as usize;
        for i in 0..height {
            if y+i > DISPLAY_HEIGHT as u8 { break; }

            let byte = self.memory[index];
            for j in 0..8 {
                // Stop drawing row if outside display bounds
                if x+j > DISPLAY_WIDTH as u8 { break; }

                // Get pixel value from sprite data in memory
                let sprite_value = ((byte >> (7 - j)) & 1) == 1;

                if sprite_value {
                    // Flip corresponding bit on screen
                    if let Some(screen_value) = self.get_pixel((x+j) as usize, (y+i) as usize) {
                        self.set_pixel((x+j) as usize, (y+i) as usize, !screen_value);

                        if screen_value { self.registers[15] = 1; }
                    }
                }
            }

            index += 1;
        }
    }

    // Debug methods
    pub fn _print_memory(&self) {
        for (i, val) in self.memory.iter().enumerate() {
            if i % 10 == 0 { println!(); }
            print!("{:3x}: {:<2x}, ", i, val);
        }
        println!("\n");
    }
}
