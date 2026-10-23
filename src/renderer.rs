use crate::{frame::Frame, palette, ppu::NesPPU};

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
    let nametable = ppu.ctrl.nametable_addr();
    let bank = ppu.ctrl.bknd_pattern_addr();

    for i in 0..960 {
        let tile_idx = ppu.vram[ppu.mirror_vram_addr(nametable + i as u16) as usize];
        let tile_column = i % 32;
        let tile_row = i / 32;
        let palette = bg_palette(ppu, nametable, tile_column, tile_row);
        draw_tile(
            ppu,
            frame,
            tile_idx,
            bank,
            tile_column * 8,
            tile_row * 8,
            palette,
            false,
            false,
            false,
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
