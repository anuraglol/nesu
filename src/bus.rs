use crate::{cartridge::Rom, ppu::NesPPU};

const RAM_MIRRORS_END: u16 = 0x1fff;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3fff;
const ROM_START: u16 = 0x8000;

pub struct Bus<'a> {
    cpu_vram: [u8; 2048],
    rom: Rom,
    ppu: NesPPU,
    cycles: usize,
    gameloop_callback: Box<dyn FnMut(&NesPPU) + 'a>,
}

impl<'a> Bus<'a> {
    pub fn new<F>(rom: Rom, gameloop_callback: F) -> Self
    where
        F: FnMut(&NesPPU) + 'a,
    {
        let ppu = NesPPU::new(rom.chr_rom.clone(), rom.screen_mirroring.clone());
        Self {
            cpu_vram: [0; 2048],
            rom,
            ppu,
            cycles: 0,
            gameloop_callback: Box::new(gameloop_callback),
        }
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.ppu.poll_nmi_status()
    }

    // todo: to change
    pub fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;

        let frame_complete = self.ppu.tick(cycles as usize * 3);

        if frame_complete {
            (self.gameloop_callback)(&self.ppu);
        }
    }

    fn read_prg_rom(&self, addr: u16) -> u8 {
        let mut offset = (addr - ROM_START) as usize;
        if self.rom.prg_rom.len() == 0x4000 {
            offset %= 0x4000;
        }
        self.rom.prg_rom[offset]
    }

    fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            0x2002 => self.ppu.read_status(),
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data(),
            _ => 0,
        }
    }

    fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            0x2000 => self.ppu.write_to_ctrl_register(data),
            0x2001 => self.ppu.write_to_mask_register(data),
            0x2003 => self.ppu.write_to_oam_addr(data),
            0x2004 => self.ppu.write_to_oam_data(data),
            0x2005 => self.ppu.write_to_scroll(data),
            0x2006 => self.ppu.write_to_ppu_addr(data),
            0x2007 => self.ppu.write_to_data(data),
            _ => {}
        }
    }

    fn dma(&mut self, page: u8) {
        let start = (page as u16) << 8;
        let mut bytes = [0u8; 256];
        for i in 0..256 {
            bytes[i] = self.read_cpu_ram_or_rom(start.wrapping_add(i as u16));
        }
        for byte in bytes {
            self.ppu.write_to_oam_data(byte);
        }
        self.tick(255);
        self.tick(255);
        self.tick(3);
    }

    fn read_cpu_ram_or_rom(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=RAM_MIRRORS_END => self.cpu_vram[(addr & 0x07ff) as usize],
            ROM_START..=0xffff => self.read_prg_rom(addr),
            _ => 0,
        }
    }
}

pub trait Mem {
    fn mem_read(&mut self, addr: u16) -> u8;
    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        u16::from_le_bytes([self.mem_read(pos), self.mem_read(pos.wrapping_add(1))])
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let [lo, hi] = data.to_le_bytes();
        self.mem_write(pos, lo);
        self.mem_write(pos.wrapping_add(1), hi);
    }
}

impl Mem for Bus<'_> {
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=RAM_MIRRORS_END => self.cpu_vram[(addr & 0x07ff) as usize],
            PPU_REGISTERS..=PPU_REGISTERS_MIRRORS_END => {
                let register = 0x2000 | (addr & 7);
                self.read_register(register)
            }
            ROM_START..=0xffff => self.read_prg_rom(addr),
            _ => 0,
        }
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=RAM_MIRRORS_END => self.cpu_vram[(addr & 0x07ff) as usize] = data,
            0x2000..=0x3fff => {
                let register = 0x2000 | (addr & 7);
                self.write_register(register, data);
            }
            0x4014 => self.dma(data),
            ROM_START..=0xffff => {}
            _ => {}
        }
    }
}
