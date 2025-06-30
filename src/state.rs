use crate::dma::DmaState;
use crate::interrupts::InterruptState;
use crate::mmu::MMU;
use crate::ppu::PpuState;
use crate::timer::TimerState;

#[derive(Debug, Clone)]
pub struct CpuState {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Default for CpuState {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuState {
    pub fn new() -> Self {
        Self {
            a: 0x01,    // A = $01
            f: 0xB0,    // F = Z=1 N=0 H=0 C=0 (Z flag set, others clear)
            b: 0x00,    // B = $00
            c: 0x13,    // C = $13
            d: 0x00,    // D = $00
            e: 0xD8,    // E = $D8
            h: 0x01,    // H = $01
            l: 0x4D,    // L = $4D
            sp: 0xFFFE, // SP = $FFFE
            pc: 0x0100, // PC = $0100
        }
    }

    // 16-bit register getters
    pub fn af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    // 16-bit register setters
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0xF0) as u8; // Lower 4 bits of F are always 0
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    // Flag operations
    pub fn flag_z(&self) -> bool {
        (self.f & 0x80) != 0
    }

    pub fn flag_n(&self) -> bool {
        (self.f & 0x40) != 0
    }

    pub fn flag_h(&self) -> bool {
        (self.f & 0x20) != 0
    }

    pub fn flag_c(&self) -> bool {
        (self.f & 0x10) != 0
    }

    pub fn set_flag_z(&mut self, value: bool) {
        if value {
            self.f |= 0x80;
        } else {
            self.f &= !0x80;
        }
    }

    pub fn set_flag_n(&mut self, value: bool) {
        if value {
            self.f |= 0x40;
        } else {
            self.f &= !0x40;
        }
    }

    pub fn set_flag_h(&mut self, value: bool) {
        if value {
            self.f |= 0x20;
        } else {
            self.f &= !0x20;
        }
    }

    pub fn set_flag_c(&mut self, value: bool) {
        if value {
            self.f |= 0x10;
        } else {
            self.f &= !0x10;
        }
    }

    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.f = 0;
        if z {
            self.f |= 0x80;
        }
        if n {
            self.f |= 0x40;
        }
        if h {
            self.f |= 0x20;
        }
        if c {
            self.f |= 0x10;
        }
    }
}

#[derive(Debug)]
pub struct EmulatorState {
    pub cpu: CpuState,
    pub mmu: MMU,
    pub ppu: PpuState,
    pub interrupts: InterruptState,
    pub timer: TimerState,
    pub dma: DmaState,
}

impl Default for EmulatorState {
    fn default() -> Self {
        Self::new()
    }
}

impl EmulatorState {
    pub fn new() -> Self {
        Self {
            cpu: CpuState::new(),
            mmu: MMU::new(),
            ppu: PpuState::new(),
            interrupts: InterruptState::new(),
            timer: TimerState::new(),
            dma: DmaState::new(),
        }
    }

    // Helper methods to access memory through the MMU
    pub fn read_memory(&self, address: u16) -> u8 {
        // Handle I/O registers specially to avoid nested borrows
        match address {
            // I/O Registers (0xFF00-0xFF7F) and IE register (0xFFFF)
            0xFF00..=0xFF7F | 0xFFFF => self.read_io_register(address),
            _ => {
                let dma_active = self.dma.is_bus_restricted();
                self.mmu.read(address, dma_active, &self.ppu)
            }
        }
    }

    pub fn write_memory(&mut self, address: u16, value: u8) {
        // Handle I/O registers specially to avoid nested borrows
        match address {
            // I/O Registers (0xFF00-0xFF7F) and IE register (0xFFFF)
            0xFF00..=0xFF7F | 0xFFFF => {
                self.write_io_register(address, value);
            }
            _ => {
                let dma_active = self.dma.is_bus_restricted();
                self.mmu.write(address, value, dma_active, &mut self.ppu);
            }
        }
    }

    // Internal I/O register handling
    fn read_io_register(&self, address: u16) -> u8 {
        match address {
            crate::io::IF_REGISTER => self.interrupts.if_reg,
            crate::io::IE_REGISTER => self.interrupts.ie,
            crate::timer::DIV_REGISTER
            | crate::timer::TIMA_REGISTER
            | crate::timer::TMA_REGISTER
            | crate::timer::TAC_REGISTER => {
                crate::timer::read_timer_register_direct(address, &self.timer)
            }
            crate::io::DMA_REGISTER => (self.dma.source_address >> 8) as u8,
            // PPU registers
            crate::ppu::LCDC_REGISTER => self.ppu.lcdc,
            crate::ppu::STAT_REGISTER => self.ppu.stat | 0x80, // Bit 7 always reads as 1
            crate::ppu::SCY_REGISTER => self.ppu.scy,
            crate::ppu::SCX_REGISTER => self.ppu.scx,
            crate::ppu::LY_REGISTER => self.ppu.ly,
            crate::ppu::LYC_REGISTER => self.ppu.lyc,
            crate::ppu::BGP_REGISTER => self.ppu.bgp,
            crate::ppu::OBP0_REGISTER => self.ppu.obp0,
            crate::ppu::OBP1_REGISTER => self.ppu.obp1,
            crate::ppu::WY_REGISTER => self.ppu.wy,
            crate::ppu::WX_REGISTER => self.ppu.wx,
            _ => self.mmu.io_registers[(address - 0xFF00) as usize],
        }
    }

    fn write_io_register(&mut self, address: u16, value: u8) {
        match address {
            crate::io::IF_REGISTER => self.interrupts.if_reg = value,
            crate::io::IE_REGISTER => self.interrupts.ie = value,
            crate::timer::DIV_REGISTER
            | crate::timer::TIMA_REGISTER
            | crate::timer::TMA_REGISTER
            | crate::timer::TAC_REGISTER => {
                crate::timer::write_timer_register_direct(address, value, &mut self.timer);
            }
            crate::io::DMA_REGISTER => self.dma.request_transfer(value),
            // PPU registers
            crate::ppu::LCDC_REGISTER => self.ppu.lcdc = value,
            crate::ppu::STAT_REGISTER => {
                // Only bits 3-6 are writable
                self.ppu.stat = (self.ppu.stat & 0x87) | (value & 0x78);
            }
            crate::ppu::SCY_REGISTER => self.ppu.scy = value,
            crate::ppu::SCX_REGISTER => self.ppu.scx = value,
            crate::ppu::LY_REGISTER => {} // LY is read-only
            crate::ppu::LYC_REGISTER => {
                self.ppu.lyc = value;
                self.ppu.update_lyc_ly_flag();
                // Check for STAT interrupt
                if self.ppu.lyc_int_select() && self.ppu.lyc_equals_ly() {
                    self.interrupts
                        .request_interrupt(crate::interrupts::InterruptType::Lcd);
                }
            }
            crate::ppu::BGP_REGISTER => self.ppu.bgp = value,
            crate::ppu::OBP0_REGISTER => self.ppu.obp0 = value,
            crate::ppu::OBP1_REGISTER => self.ppu.obp1 = value,
            crate::ppu::WY_REGISTER => self.ppu.wy = value,
            crate::ppu::WX_REGISTER => self.ppu.wx = value,
            _ => {
                if address >= 0xFF00 && address <= 0xFF7F {
                    self.mmu.io_registers[(address - 0xFF00) as usize] = value;
                }
            }
        }
    }

    // Cartridge management
    pub fn load_cartridge(&mut self, rom_data: Vec<u8>) -> Result<(), String> {
        self.mmu.load_cartridge(rom_data)
    }

    pub fn remove_cartridge(&mut self) {
        self.mmu.remove_cartridge();
    }

    pub fn get_cartridge_info(&self) -> Option<&crate::cartridge::CartridgeInfo> {
        self.mmu.get_cartridge_info()
    }
}
