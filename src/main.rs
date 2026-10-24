mod bus;
pub mod cartridge;
pub mod cpu;
pub mod frame;
mod instructions;
pub mod joypad;
pub mod opcodes;
pub mod palette;
pub mod ppu;
mod renderer;

use cartridge::Rom;
use cpu::CPU;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};

const FRAME_TIME: Duration = Duration::from_nanos(16_639_000); // ~60.0988 Hz

fn main() {
    let file_bytes = fs::read("./roms/pacman.nes").expect("failed to read rom file");
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

    let mut key_map = HashMap::new();
    key_map.insert(Keycode::Down, joypad::JoypadButton::DOWN);
    key_map.insert(Keycode::Up, joypad::JoypadButton::UP);
    key_map.insert(Keycode::Right, joypad::JoypadButton::RIGHT);
    key_map.insert(Keycode::Left, joypad::JoypadButton::LEFT);
    key_map.insert(Keycode::Space, joypad::JoypadButton::SELECT);
    key_map.insert(Keycode::Return, joypad::JoypadButton::START);
    key_map.insert(Keycode::A, joypad::JoypadButton::BUTTON_A);
    key_map.insert(Keycode::S, joypad::JoypadButton::BUTTON_B);

    canvas.clear();
    canvas.present();

    let bus = bus::Bus::new(rom, |ppu: &ppu::NesPPU, joypad: &mut joypad::Joypad| {
        let frame_start = Instant::now();

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

                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(key) = key_map.get(&keycode) {
                        joypad.set_button_pressed_status(*key, true);
                    }
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(key) = key_map.get(&keycode) {
                        joypad.set_button_pressed_status(*key, false);
                    }
                }

                _ => { /* do nothing */ }
            }
        }

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_TIME {
            std::thread::sleep(FRAME_TIME - elapsed);
        }
    });

    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.run();
}
