use crate::cartridge::Mirroring;
use bitflags::bitflags;

pub struct NesPPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 32],
    pub vram: [u8; 2048],
    pub oam_data: [u8; 256],
    pub oam_addr: u8,
    pub mirroring: Mirroring,
    pub addr: AddrRegister,
    pub ctrl: ControlRegister,
    pub internal_data_buf: u8,
    pub status: StatusRegister,
    pub mask: MaskRegister,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub scanline: u16,
    pub cycles: usize,
    pub nmi_interrupt: Option<u8>,
}

impl NesPPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Self {
            chr_rom,
            mirroring,
            vram: [0; 2048],
            oam_data: [0; 256],
            palette_table: [0; 32],
            addr: AddrRegister::new(),
            ctrl: ControlRegister::new(),
            internal_data_buf: 0,
            oam_addr: 0,
            status: StatusRegister::empty(),
            mask: MaskRegister::empty(),
            scroll_x: 0,
            scroll_y: 0,
            scanline: 0,
            cycles: 0,
            nmi_interrupt: None,
        }
    }

    fn write_to_ctrl(&mut self, value: u8) {
        let was_enabled = self.ctrl.generate_vblank_nmi();
        self.ctrl.update(value);
        if !was_enabled && self.ctrl.generate_vblank_nmi() && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(1);
        }
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }

    pub fn tick(&mut self, cycles: usize) -> bool {
        self.cycles += cycles;
        let mut frame_complete = false;

        while self.cycles >= 341 {
            self.cycles -= 341;
            self.scanline += 1;

            if self.scanline == 241 {
                self.status.insert(StatusRegister::VBLANK_STARTED);
                if self.ctrl.generate_vblank_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }

            if self.scanline >= 262 {
                self.scanline = 0;
                self.status.remove(StatusRegister::VBLANK_STARTED);
                self.status
                    .remove(StatusRegister::SPRITE_ZERO_HIT | StatusRegister::SPRITE_OVERFLOW);
                frame_complete = true;
            }
        }

        frame_complete
    }

    pub fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    pub fn write_to_ctrl_register(&mut self, value: u8) {
        self.write_to_ctrl(value);
    }

    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }

    pub fn write_to_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    pub fn read_status(&mut self) -> u8 {
        let result = self.status.bits();
        self.status.remove(StatusRegister::VBLANK_STARTED);
        self.addr.reset_latch();
        result
    }

    pub fn write_to_mask_register(&mut self, value: u8) {
        self.mask = MaskRegister::from_bits_retain(value);
    }

    pub fn write_to_scroll(&mut self, value: u8) {
        if self.addr.is_hi_ptr() {
            self.scroll_x = value;
        } else {
            self.scroll_y = value;
        }
        self.addr.toggle_latch();
    }

    pub fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.increment_vram_addr();
        match addr {
            0x0000..=0x1fff => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x2fff | 0x3000..=0x3eff => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
            0x3f00..=0x3fff => self.palette_table[Self::mirror_palette_addr(addr) as usize],
            _ => 0,
        }
    }

    fn mirror_palette_addr(addr: u16) -> usize {
        let mut idx = ((addr - 0x3f00) % 32) as usize;
        if matches!(idx, 0x10 | 0x14 | 0x18 | 0x1c) {
            idx -= 0x10;
        }
        idx
    }

    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0x2fff;
        let vram_index = mirrored_vram - 0x2000;
        let name_table = vram_index / 0x400;
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            (Mirroring::FourScreen, _) => vram_index,
            _ => vram_index,
        }
    }

    pub fn write_to_data(&mut self, value: u8) {
        let addr = self.addr.get();
        self.increment_vram_addr();
        match addr {
            0x0000..=0x1fff => {
                if let Some(byte) = self.chr_rom.get_mut(addr as usize) {
                    *byte = value;
                }
            }
            0x2000..=0x2fff | 0x3000..=0x3eff => {
                let mirrored_addr = self.mirror_vram_addr(addr);
                self.vram[mirrored_addr as usize] = value;
            }
            0x3f00..=0x3fff => {
                let palette_idx = Self::mirror_palette_addr(addr);
                self.palette_table[palette_idx] = value & 0x3f;
            }
            _ => {}
        }
    }
}

pub struct AddrRegister {
    value: (u8, u8),
    hi_ptr: bool,
}

impl AddrRegister {
    pub fn new() -> Self {
        Self {
            value: (0, 0),
            hi_ptr: true,
        }
    }

    fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = data as u8;
    }

    pub fn is_hi_ptr(&self) -> bool {
        self.hi_ptr
    }

    pub fn toggle_latch(&mut self) {
        self.hi_ptr = !self.hi_ptr;
    }

    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.value.0 = data & 0x3f;
        } else {
            self.value.1 = data;
        }
        self.set(self.get() & 0x3fff);
        self.hi_ptr = !self.hi_ptr;
    }

    pub fn increment(&mut self, inc: u8) {
        self.set(self.get().wrapping_add(inc as u16) & 0x3fff);
    }

    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    pub fn get(&self) -> u16 {
        ((self.value.0 as u16) << 8) | self.value.1 as u16
    }
}

bitflags! {
    pub struct ControlRegister: u8 {
        const NAMETABLE1 = 0b00000001;
        const NAMETABLE2 = 0b00000010;
        const VRAM_ADD_INCREMENT = 0b00000100;
        const SPRITE_PATTERN_ADDR = 0b00001000;
        const BACKROUND_PATTERN_ADDR = 0b00010000;
        const SPRITE_SIZE = 0b00100000;
        const MASTER_SLAVE_SELECT = 0b01000000;
        const GENERATE_NMI = 0b10000000;
    }

    pub struct StatusRegister: u8 {
        const SPRITE_OVERFLOW = 0b0010_0000;
        const SPRITE_ZERO_HIT = 0b0100_0000;
        const VBLANK_STARTED = 0b1000_0000;
    }

    pub struct MaskRegister: u8 {
        const GREYSCALE = 0b0000_0001;
        const SHOW_BACKGROUND_LEFT = 0b0000_0010;
        const SHOW_SPRITES_LEFT = 0b0000_0100;
        const SHOW_BACKGROUND = 0b0000_1000;
        const SHOW_SPRITES = 0b0001_0000;
        const EMPHASIZE_RED = 0b0010_0000;
        const EMPHASIZE_GREEN = 0b0100_0000;
        const EMPHASIZE_BLUE = 0b1000_0000;
    }
}

impl ControlRegister {
    pub fn new() -> Self {
        Self::empty()
    }

    pub fn vram_addr_increment(&self) -> u8 {
        if self.contains(Self::VRAM_ADD_INCREMENT) {
            32
        } else {
            1
        }
    }

    pub fn update(&mut self, data: u8) {
        *self = Self::from_bits_retain(data);
    }

    pub fn generate_vblank_nmi(&self) -> bool {
        self.contains(Self::GENERATE_NMI)
    }

    pub fn bknd_pattern_addr(&self) -> u16 {
        if self.contains(Self::BACKROUND_PATTERN_ADDR) {
            0x1000
        } else {
            0
        }
    }

    pub fn sprt_pattern_addr(&self) -> u16 {
        if self.contains(Self::SPRITE_PATTERN_ADDR) {
            0x1000
        } else {
            0
        }
    }

    pub fn nametable_addr(&self) -> u16 {
        match self.bits() & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            _ => 0x2c00,
        }
    }

    pub fn sprite_size_16(&self) -> bool {
        self.contains(Self::SPRITE_SIZE)
    }
}

impl StatusRegister {
    pub fn is_in_vblank(&self) -> bool {
        self.contains(Self::VBLANK_STARTED)
    }
}
