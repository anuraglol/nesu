use crate::{frame::Frame, palette, ppu::NesPPU};

#[derive(Clone, Copy)]
struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Rect {
    fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Self { x1, y1, x2, y2 }
    }
}

fn system_color(index: u8) -> (u8, u8, u8) {
    palette::SYSTEM_PALLETE[(index & 0x3f) as usize]
}

fn bg_palette(ppu: &NesPPU, name_table: u16, tile_column: usize, tile_row: usize) -> [u8; 4] {
    let attr_table_idx = (tile_row / 4) * 8 + tile_column / 4;
    let attr_addr = name_table + 0x3c0 + attr_table_idx as u16;
    let attr_byte = ppu.vram[ppu.mirror_vram_addr(attr_addr) as usize];
    let shift = match (tile_column % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => 0,
        (1, 0) => 2,
        (0, 1) => 4,
        (1, 1) => 6,
        _ => unreachable!(),
    };
    let palette_idx = ((attr_byte >> shift) & 3) as usize;
    let start = 1 + palette_idx * 4;
    [
        ppu.palette_table[0],
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}

fn sprite_palette(ppu: &NesPPU, palette_idx: u8) -> [u8; 4] {
    let start = 0x11 + palette_idx as usize * 4;
    [
        0,
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}

fn render_name_table(
    ppu: &NesPPU,
    frame: &mut Frame,
    name_table: &[u8],
    name_table_addr: u16,
    view_port: Rect,
    shift_x: isize,
    shift_y: isize,
) {
    let bank = ppu.ctrl.bknd_pattern_addr();

    for i in 0..0x3c0 {
        let tile_column = i % 32;
        let tile_row = i / 32;
        let tile_idx = name_table[i] as usize;
        let tile_start = bank as usize + tile_idx * 16;
        let tile = &ppu.chr_rom[tile_start..tile_start + 16];
        let palette = bg_palette(ppu, name_table_addr, tile_column, tile_row);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = ((lower & 1) << 1) | (upper & 1);
                upper >>= 1;
                lower >>= 1;
                let rgb = match value {
                    0 => system_color(ppu.palette_table[0]),
                    1 => system_color(palette[1]),
                    2 => system_color(palette[2]),
                    3 => system_color(palette[3]),
                    _ => panic!("can't be"),
                };
                let pixel_x = tile_column * 8 + x;
                let pixel_y = tile_row * 8 + y;

                if pixel_x >= view_port.x1 && pixel_x < view_port.x2 && pixel_y >= view_port.y1 && pixel_y < view_port.y2 {
                    frame.set_pixel(
                        (shift_x + pixel_x as isize) as usize,
                        (shift_y + pixel_y as isize) as usize,
                        rgb,
                    );
                }
            }
        }
    }
}

fn draw_tile(
    ppu: &NesPPU,
    frame: &mut Frame,
    tile_idx: u8,
    bank: u16,
    x0: usize,
    y0: usize,
    palette: [u8; 4],
    transparent_zero: bool,
    flip_horizontal: bool,
    flip_vertical: bool,
) {
    let start = bank as usize + tile_idx as usize * 16;
    if start + 16 > ppu.chr_rom.len() {
        return;
    }
    let tile = &ppu.chr_rom[start..start + 16];
    for y in 0..8 {
        let plane0 = tile[y];
        let plane1 = tile[y + 8];
        for x in 0..8 {
            let bit = 7 - x;
            let value = ((plane1 >> bit) & 1) << 1 | ((plane0 >> bit) & 1);
            if transparent_zero && value == 0 {
                continue;
            }
            let px = if flip_horizontal { 7 - x } else { x };
            let py = if flip_vertical { 7 - y } else { y };
            let color_index = if value == 0 {
                ppu.palette_table[0]
            } else {
                palette[value as usize]
            };
            frame.set_pixel(x0 + px, y0 + py, system_color(color_index));
        }
    }
}

pub fn render(ppu: &NesPPU, frame: &mut Frame) {
    let scroll_x = ppu.scroll_x as usize;
    let scroll_y = ppu.scroll_y as usize;
    let (main_nametable, second_nametable) = match (&ppu.mirroring, ppu.ctrl.nametable_addr()) {
        (crate::cartridge::Mirroring::Vertical, 0x2000)
        | (crate::cartridge::Mirroring::Vertical, 0x2800) => (0x2000, 0x2400),
        (crate::cartridge::Mirroring::Vertical, 0x2400)
        | (crate::cartridge::Mirroring::Vertical, 0x2c00) => (0x2400, 0x2000),
        (crate::cartridge::Mirroring::Horizontal, 0x2000)
        | (crate::cartridge::Mirroring::Horizontal, 0x2400) => (0x2000, 0x2400),
        (crate::cartridge::Mirroring::Horizontal, 0x2800)
        | (crate::cartridge::Mirroring::Horizontal, 0x2c00) => (0x2800, 0x2000),
        (crate::cartridge::Mirroring::FourScreen, _) => (ppu.ctrl.nametable_addr(), 0x2000),
        _ => panic!("Not supported mirroring type {:?}", ppu.mirroring),
    };

    if scroll_x > 0 {
        let main_start = ppu.mirror_vram_addr(main_nametable) as usize;
        let second_start = ppu.mirror_vram_addr(second_nametable) as usize;
        render_name_table(
            ppu,
            frame,
            &ppu.vram[main_start..main_start + 0x400],
            main_nametable,
            Rect::new(scroll_x, scroll_y, 256, 240),
            -(scroll_x as isize),
            -(scroll_y as isize),
        );
        render_name_table(
            ppu,
            frame,
            &ppu.vram[second_start..second_start + 0x400],
            second_nametable,
            Rect::new(0, 0, scroll_x, 240),
            (256 - scroll_x) as isize,
            0,
        );
    } else if scroll_y > 0 {
        let main_start = ppu.mirror_vram_addr(main_nametable) as usize;
        let second_start = ppu.mirror_vram_addr(second_nametable) as usize;
        render_name_table(
            ppu,
            frame,
            &ppu.vram[main_start..main_start + 0x400],
            main_nametable,
            Rect::new(0, scroll_y, 256, 240),
            0,
            -(scroll_y as isize),
        );
        render_name_table(
            ppu,
            frame,
            &ppu.vram[second_start..second_start + 0x400],
            second_nametable,
            Rect::new(0, 0, 256, scroll_y),
            0,
            (240 - scroll_y) as isize,
        );
    } else {
        let nametable = ppu.ctrl.nametable_addr();
        let start = ppu.mirror_vram_addr(nametable) as usize;
        render_name_table(
            ppu,
            frame,
            &ppu.vram[start..start + 0x400],
            nametable,
            Rect::new(0, 0, 256, 240),
            0,
            0,
        );
    }

    let sprite_bank = ppu.ctrl.sprt_pattern_addr();
    for i in (0..ppu.oam_data.len()).step_by(4).rev() {
        let y = ppu.oam_data[i] as usize;
        let tile_idx = ppu.oam_data[i + 1];
        let attributes = ppu.oam_data[i + 2];
        let x = ppu.oam_data[i + 3] as usize;
        if y >= 240 || x >= 256 {
            continue;
        }
        let palette = sprite_palette(ppu, attributes & 3);
        draw_tile(
            ppu,
            frame,
            tile_idx,
            sprite_bank,
            x,
            y + 1,
            palette,
            true,
            attributes & 0x40 != 0,
            attributes & 0x80 != 0,
        );
    }
}
