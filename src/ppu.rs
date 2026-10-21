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
        NesPPU {
            chr_rom: chr_rom,
            mirroring: mirroring,
            vram: [0; 2048],
            oam_data: [0; 64 * 4],
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
        let before_nmi_status = self.ctrl.generate_vblank_nmi();
        self.ctrl.update(value);
        if !before_nmi_status && self.ctrl.generate_vblank_nmi() && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(1);
        }
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.nmi_interrupt.take() // .take() takes the Some and leaves None, useful to consume the state only once
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        self.cycles += cycles as usize;

        let mut frame_complete = false;

        while self.cycles >= 341 {
            if self.cycles == 341 {
                self.status.insert(StatusRegister::VBLANK_STARTED);

                if self.ctrl.generate_vblank_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }

            self.cycles -= 341;
            self.scanline += 1;

            if self.scanline >= 262 {
                self.scanline = 0;
                frame_complete = true;
            }
        }

        frame_complete
    }

    pub fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    pub fn write_to_ctrl_register(&mut self, value: u8) {
        self.ctrl.update(value);
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
        return self.oam_data[self.oam_addr as usize];
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
            0..=0x1fff => {
                // read from chr_rom
                let result = self.internal_data_buf;
                self.internal_data_buf = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x2fff => {
                // read from ram
                let result = self.internal_data_buf;
                self.internal_data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
            0x3000..=0x3eff => panic!(
                "addr space 0x3000..0x3eff is not expected to be used, requested = {} ",
                addr
            ),
            0x3f00..=0x3fff => self.palette_table[(addr - 0x3f00) as usize],
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }

    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400; // to the name table index
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    pub fn write_to_data(&mut self, value: u8) {
        let addr = self.addr.get();
        self.increment_vram_addr();

        match addr {
            0..=0x1fff => {
                self.chr_rom[addr as usize] = value;
            }
            0x2000..=0x2fff => {
                let mirrored_addr = self.mirror_vram_addr(addr);
                self.vram[mirrored_addr as usize] = value;
            }
            0x3000..=0x3eff => {
                let mirrored_addr = self.mirror_vram_addr(addr);
                self.vram[mirrored_addr as usize] = value;
            }
            0x3f00..=0x3fff => {
                let mut palette_idx = (addr - 0x3f00) % 32;
                match palette_idx {
                    0x10 => palette_idx = 0x00,
                    0x14 => palette_idx = 0x04,
                    0x18 => palette_idx = 0x08,
                    0x1c => palette_idx = 0x0c,
                    _ => {}
                }
                self.palette_table[palette_idx as usize] = value;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }
}

pub struct AddrRegister {
    value: (u8, u8),
    hi_ptr: bool,
}

impl AddrRegister {
    pub fn new() -> Self {
        AddrRegister {
            value: (0, 0), // high byte first, lo byte second
            hi_ptr: true,
        }
    }
    fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = (data & 0xff) as u8;
    }

    pub fn is_hi_ptr(&self) -> bool {
        self.hi_ptr
    }

    pub fn toggle_latch(&mut self) {
        self.hi_ptr = !self.hi_ptr;
    }

    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.value.0 = data;
        } else {
            self.value.1 = data;
        }

        if self.get() > 0x3fff {
            //mirror down addr above 0x3fff
            self.set(self.get() & 0b11111111111111);
        }
        self.hi_ptr = !self.hi_ptr;
    }

    pub fn increment(&mut self, inc: u8) {
        let lo = self.value.1;
        self.value.1 = self.value.1.wrapping_add(inc);
        if lo > self.value.1 {
            self.value.0 = self.value.0.wrapping_add(1);
        }
        if self.get() > 0x3fff {
            self.set(self.get() & 0b11111111111111); //mirror down addr above 0x3fff
        }
    }

    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    pub fn get(&self) -> u16 {
        ((self.value.0 as u16) << 8) | (self.value.1 as u16)
    }
}

bitflags! {

   // 7  bit  0
   // ---- ----
   // VPHB SINN
   // |||| ||||
   // |||| ||++- Base nametable address
   // |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
   // |||| |+--- VRAM address increment per CPU read/write of PPUDATA
   // |||| |     (0: add 1, going across; 1: add 32, going down)
   // |||| +---- Sprite pattern table address for 8x8 sprites
   // ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
   // |||+------ Background pattern table address (0: $0000; 1: $1000)
   // ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels)
   // |+-------- PPU master/slave select
   // |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
   // +--------- Generate an NMI at the start of the
   //            vertical blanking interval (0: off; 1: on)
   pub struct ControlRegister: u8 {
       const NAMETABLE1              = 0b00000001;
       const NAMETABLE2              = 0b00000010;
       const VRAM_ADD_INCREMENT      = 0b00000100;
       const SPRITE_PATTERN_ADDR     = 0b00001000;
       const BACKROUND_PATTERN_ADDR  = 0b00010000;
       const SPRITE_SIZE             = 0b00100000;
       const MASTER_SLAVE_SELECT     = 0b01000000;
       const GENERATE_NMI            = 0b10000000;
   }

   pub struct StatusRegister: u8 {
           const SPRITE_OVERFLOW = 0b0010_0000;
           const SPRITE_ZERO_HIT = 0b0100_0000;
           const VBLANK_STARTED  = 0b1000_0000;
    }

    pub struct MaskRegister: u8 {
            const GREYSCALE             = 0b0000_0001;
            const SHOW_BACKGROUND_LEFT  = 0b0000_0010;
            const SHOW_SPRITES_LEFT     = 0b0000_0100;
            const SHOW_BACKGROUND       = 0b0000_1000;
            const SHOW_SPRITES          = 0b0001_0000;
            const EMPHASIZE_RED         = 0b0010_0000;
            const EMPHASIZE_GREEN       = 0b0100_0000;
            const EMPHASIZE_BLUE        = 0b1000_0000;
        }
}

impl ControlRegister {
    pub fn new() -> Self {
        ControlRegister::from_bits_truncate(0b00000000)
    }

    pub fn vram_addr_increment(&self) -> u8 {
        if !self.contains(ControlRegister::VRAM_ADD_INCREMENT) {
            1
        } else {
            32
        }
    }

    pub fn update(&mut self, data: u8) {
        *self = Self::from_bits_retain(data);
    }

    pub fn generate_vblank_nmi(&self) -> bool {
        self.contains(ControlRegister::GENERATE_NMI)
    }
}

impl StatusRegister {
    pub fn is_in_vblank(&self) -> bool {
        self.contains(StatusRegister::VBLANK_STARTED)
    }
}
