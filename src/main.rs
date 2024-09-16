use std::{env, fs, process, thread, time};
use minifb::{Key, Window, WindowOptions};
use rodio::{OutputStream, Sink};
use rodio::source::{SineWave, Source};

mod chip8;

const CYCLES_PER_SECOND: f32 = 700.0;
const MICROSECONDS_PER_CYCLE: u128 = ((1.0 / CYCLES_PER_SECOND) * 1_000_000.0) as u128;

const INPUT_MAP: [Key; 16] = [
    Key::X, /* 0 */
    Key::Key1, /* 1 */
    Key::Key2, /* 2 */
    Key::Key3, /* 3 */
    Key::Q, /* 4 */
    Key::W, /* 5 */
    Key::E, /* 6 */
    Key::A, /* 7 */
    Key::S, /* 8 */
    Key::D, /* 9 */
    Key::Z, /* A */
    Key::C, /* B */
    Key::Key4, /* C */
    Key::R, /* D */
    Key::F, /* E */
    Key::V, /* F */
    ];

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = parse_arguments(&args).unwrap_or_else(|err| {
        println!("{err}");
        process::exit(1);
    });

    let mut chip8 = chip8::Chip8::new();

    let mut window = initialize_window();

    let path = format!("roms/{}", config.rom_path);
    let program = fs::read(&path).unwrap_or_else(|_e| {
        println!("Error: file not found at {path}");
        process::exit(1);
    });
    chip8.load_to_memory(&program, 0x200);
    //chip8.print_memory();

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut display_timer = time::SystemTime::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Get keyboard input and send to chip8
        for i in 0..INPUT_MAP.len() {
            chip8.set_key(i, window.is_key_down(INPUT_MAP[i]));
        }

        // Call next chip8 CPU cycle
        chip8.update();

        if display_timer.elapsed().unwrap().as_micros() > 16600 {
            // Print FPS to console
            //println!("{}", 1.0 / display_timer.elapsed().unwrap().as_secs_f64());

            // Call chip8 draw phase
            chip8.draw();

            // Update window
            let buffer = translate_display(chip8.get_display());
            window
                .update_with_buffer(&buffer, chip8::DISPLAY_WIDTH, chip8::DISPLAY_HEIGHT)
                .unwrap();

            // Play audio
            if sink.len() <= 1 { sink.append( SineWave::new(440.0).take_duration(time::Duration::from_secs_f32(0.25)) ) };
            if chip8.get_sound_timer() > 0 { sink.play(); }
            else { sink.pause(); }

            // Reset display timer
            display_timer = time::SystemTime::now();
        }

        thread::sleep(time::Duration::from_micros(MICROSECONDS_PER_CYCLE as u64));
    }
}

fn initialize_window() -> Window {
    let mut window = Window::new(
        "chip8-rs",
        chip8::DISPLAY_WIDTH,
        chip8::DISPLAY_HEIGHT,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X8,
            ..WindowOptions::default()
        }
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.limit_update_rate(None);

    window
}

// Translates the chip8's monochrome display buffer to a buffer that can be sent to minifb
fn translate_display(chip8_buffer: &[[bool; chip8::DISPLAY_WIDTH]; chip8::DISPLAY_HEIGHT])
    -> [u32; chip8::DISPLAY_WIDTH * chip8::DISPLAY_HEIGHT] {
    let mut window_buffer = [0; chip8::DISPLAY_WIDTH * chip8::DISPLAY_HEIGHT];

    for i in 0..chip8_buffer.len() {
        for j in 0..chip8_buffer[i].len() {
            window_buffer[j + (i * chip8::DISPLAY_WIDTH)] = if chip8_buffer[i][j] == true {
                0xFFFFFFFF
            } else {
                0x00000000
            };
        }
    }

    window_buffer
}

fn parse_arguments(args: &[String]) -> Result<Config, &'static str> {
    if args.len() < 2 {
        return Err("Error: no ROM path specified");
    }

    let rom_path = args[1].clone();

    Ok(Config { rom_path })
}

struct Config {
    rom_path: String,
}

