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
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::fs;

fn main() {
    let file_bytes = fs::read("pacman.nes").expect("Failed to read pacman.nes");
    let rom = Rom::new(&file_bytes).expect("Failed to parse ROM");
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("NES Emulator", 256 * 3, 240 * 3)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();
    let mut frame = frame::Frame::new();
    let mut event_pump = sdl_context.event_pump().unwrap();

    canvas.clear();
    canvas.present();

    let bus = bus::Bus::new(rom, |ppu: &ppu::NesPPU| {
        renderer::render(ppu, &mut frame);
        texture
            .update(None, &frame.data, frame::Frame::WIDTH * 3)
            .unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                _ => {}
            }
        }
    });

    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.run();
}
