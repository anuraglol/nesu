mod bus;
pub mod cartridge;
pub mod cpu;
pub mod frame;
mod instructions;
pub mod opcodes;
pub mod palette;
pub mod ppu;
mod renderer;
mod trace;

use cartridge::Rom;
use cpu::CPU;
use cpu::Mem;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::fs;

fn handle_user_input(cpu: &mut CPU, event_pump: &mut EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => std::process::exit(0),
            Event::KeyDown {
                keycode: Some(Keycode::W),
                ..
            } => {
                cpu.mem_write(0xff, 0x77);
            }
            Event::KeyDown {
                keycode: Some(Keycode::S),
                ..
            } => {
                cpu.mem_write(0xff, 0x73);
            }
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => {
                cpu.mem_write(0xff, 0x61);
            }
            Event::KeyDown {
                keycode: Some(Keycode::D),
                ..
            } => {
                cpu.mem_write(0xff, 0x64);
            }
            _ => {}
        }
    }
}

fn show_tile(chr_rom: &[u8], bank: usize) -> frame::Frame {
    assert!(bank <= 1);

    let mut frame = frame::Frame::new();
    let bank_start = bank * 0x1000;

    for tile_n in 0..256 {
        let tile_start = bank_start + tile_n * 16;
        let tile = &chr_rom[tile_start..tile_start + 16];

        let tile_x = (tile_n % 16) * 8;
        let tile_y = (tile_n / 16) * 8;

        for y in 0..8 {
            let upper = tile[y];
            let lower = tile[y + 8];

            for x in 0..8 {
                let bit = 7 - x;
                let value = ((upper >> bit) & 1) | (((lower >> bit) & 1) << 1);

                let rgb = match value {
                    0 => palette::SYSTEM_PALLETE[0x01],
                    1 => palette::SYSTEM_PALLETE[0x23],
                    2 => palette::SYSTEM_PALLETE[0x27],
                    3 => palette::SYSTEM_PALLETE[0x30],
                    _ => unreachable!(),
                };

                frame.set_pixel(tile_x + x, tile_y + y, rgb);
            }
        }
    }

    frame
}

fn main() {
    let file_bytes = fs::read("pacman.nes").expect("Failed to read pacman.nes");

    let rom = Rom::new(&file_bytes).expect("Failed to parse ROM");

    let frame = show_tile(&rom.chr_rom, 0);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Pac-Man CHR Tiles", 128 * 5, 128 * 5)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_scale(5.0, 5.0).unwrap();

    let creator = canvas.texture_creator();

    let mut texture = creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 128, 128)
        .unwrap();

    texture.update(None, &frame.data, 128 * 3).unwrap();

    canvas.clear();
    canvas.copy(&texture, None, None).unwrap();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
