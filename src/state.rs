use crate::apu::ApuState;
use crate::dma::DmaState;
use crate::interrupts::InterruptState;
use crate::joypad::JoypadState;
use crate::mmu::MMU;
use crate::ppu::PpuState;
use crate::timer::TimerState;

// I/O register addresses
pub const P1_REGISTER: u16 = 0xFF00; // Joypad
pub const IF_REGISTER: u16 = 0xFF0F; // Interrupt Flag
pub const IE_REGISTER: u16 = 0xFFFF; // Interrupt Enable
pub const DMA_REGISTER: u16 = 0xFF46; // OAM DMA Transfer

/// Tracks which debug windows need to be re-rendered
#[derive(Debug, Clone)]
pub struct RenderDirtyFlags {
    /// Background tilemap 0 data has changed
    pub bg_tilemap_0: bool,
    /// Background tilemap 1 data has changed
    pub bg_tilemap_1: bool,
    /// VRAM tile data has changed
    pub tile_data: bool,
    /// Palette registers have changed
    pub palettes: bool,
    /// OAM sprite data has changed
    pub sprite_tiles: bool,
}

impl Default for RenderDirtyFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderDirtyFlags {
    pub fn new() -> Self {
        Self {
            // Start with all windows marked as dirty to ensure initial render
            bg_tilemap_0: true,
            bg_tilemap_1: true,
            tile_data: true,
            palettes: true,
            sprite_tiles: true,
        }
    }

    /// Mark all flags as dirty (useful for debug mode or major state changes)
    pub fn mark_all_dirty(&mut self) {
        self.bg_tilemap_0 = true;
        self.bg_tilemap_1 = true;
        self.tile_data = true;
        self.palettes = true;
        self.sprite_tiles = true;
    }

    /// Clear all dirty flags (called after successful rendering)
    pub fn clear_all(&mut self) {
        self.bg_tilemap_0 = false;
        self.bg_tilemap_1 = false;
        self.tile_data = false;
        self.palettes = false;
        self.sprite_tiles = false;
    }

    /// Check if any flags are dirty
    pub fn is_any_dirty(&self) -> bool {
        self.bg_tilemap_0
            || self.bg_tilemap_1
            || self.tile_data
            || self.palettes
            || self.sprite_tiles
    }

    /// Mark VRAM-related flags as dirty
    pub fn mark_vram_dirty(&mut self) {
        self.tile_data = true;
        self.bg_tilemap_0 = true;
        self.bg_tilemap_1 = true;
    }

    /// Mark OAM-related flags as dirty
    pub fn mark_oam_dirty(&mut self) {
        self.sprite_tiles = true;
    }

    /// Mark palette-related flags as dirty
    pub fn mark_palette_dirty(&mut self) {
        self.palettes = true;
        // Palette changes affect all rendered content
        self.bg_tilemap_0 = true;
        self.bg_tilemap_1 = true;
        self.tile_data = true;
        self.sprite_tiles = true;
    }
}

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
    pub halt_bug: bool,
    pub halted: bool,
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
            halt_bug: false,
            halted: false,
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
    pub joypad: JoypadState,
    pub apu: ApuState,
    pub debug_mode: bool,
    pub opcode_log: Vec<(u16, u8)>, // (PC, opcode)
    pub render_dirty_flags: RenderDirtyFlags,
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
            joypad: JoypadState::new(),
            apu: ApuState::new(),
            debug_mode: false,
            opcode_log: Vec::new(),
            render_dirty_flags: RenderDirtyFlags::new(),
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
                self.mmu.write(
                    address,
                    value,
                    dma_active,
                    &mut self.ppu,
                    &mut self.render_dirty_flags,
                );
            }
        }
    }

    // Internal I/O register handling
    fn read_io_register(&self, address: u16) -> u8 {
        match address {
            P1_REGISTER => {
                // Calculate P1 value based on current joypad state and selection
                let p1_select = self.mmu.io_registers[0x00];
                self.joypad.calculate_p1(p1_select) | 0xC0 // Bits 6-7 always 1
            }
            IF_REGISTER => self.interrupts.if_reg | 0xE0,
            IE_REGISTER => self.interrupts.ie,
            crate::timer::DIV_REGISTER
            | crate::timer::TIMA_REGISTER
            | crate::timer::TMA_REGISTER
            | crate::timer::TAC_REGISTER => {
                crate::timer::read_timer_register_direct(address, &self.timer)
            }
            DMA_REGISTER => self.dma.last_written_value,
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
            // APU registers
            crate::apu::NR10_REGISTER..=crate::apu::NR52_REGISTER => {
                crate::apu::read_apu_register(address, &self.apu)
            }
            _ => self.mmu.io_registers[(address - 0xFF00) as usize],
        }
    }

    fn write_io_register(&mut self, address: u16, value: u8) {
        match address {
            P1_REGISTER => {
                // Only bits 4-5 are writable for P1 register (selection bits)
                // Store the selection bits, the lower nibble will be calculated on read
                self.mmu.io_registers[0x00] = (value & 0x30) | 0xC0; // Keep bits 6-7 high, bits 0-3 are calculated on read
            }
            IF_REGISTER => self.interrupts.if_reg = (value & 0x1F) | 0xE0,
            IE_REGISTER => self.interrupts.ie = value,
            crate::timer::DIV_REGISTER
            | crate::timer::TIMA_REGISTER
            | crate::timer::TMA_REGISTER
            | crate::timer::TAC_REGISTER => {
                crate::timer::write_timer_register_direct(address, value, &mut self.timer);
            }
            DMA_REGISTER => self.dma.request_transfer(value),
            // PPU registers
            crate::ppu::LCDC_REGISTER => self.ppu.lcdc = value,
            crate::ppu::STAT_REGISTER => {
                // Only bits 3-6 are writable
                self.ppu.stat = (self.ppu.stat & 0x87) | (value & 0x78);

                // Check if this write causes a STAT interrupt
                if self.ppu.check_and_trigger_stat_interrupt() {
                    self.interrupts
                        .request_interrupt(crate::interrupts::InterruptType::Lcd);
                }
            }
            crate::ppu::SCY_REGISTER => self.ppu.scy = value,
            crate::ppu::SCX_REGISTER => self.ppu.scx = value,
            crate::ppu::LY_REGISTER => {} // LY is read-only
            crate::ppu::LYC_REGISTER => {
                self.ppu.lyc = value;
                self.ppu.update_lyc_ly_flag();

                // Check for STAT interrupt with blocking
                if self.ppu.check_and_trigger_stat_interrupt() {
                    self.interrupts
                        .request_interrupt(crate::interrupts::InterruptType::Lcd);
                }
            }
            crate::ppu::BGP_REGISTER => {
                self.ppu.bgp = value;
                self.render_dirty_flags.mark_palette_dirty();
            }
            crate::ppu::OBP0_REGISTER => {
                self.ppu.obp0 = value;
                self.render_dirty_flags.mark_palette_dirty();
            }
            crate::ppu::OBP1_REGISTER => {
                self.ppu.obp1 = value;
                self.render_dirty_flags.mark_palette_dirty();
            }
            crate::ppu::WY_REGISTER => self.ppu.wy = value,
            crate::ppu::WX_REGISTER => self.ppu.wx = value,
            // APU registers
            crate::apu::NR10_REGISTER..=crate::apu::NR52_REGISTER => {
                crate::apu::write_apu_register(address, value, &mut self.apu)
            }
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

    // Dirty flag management methods
    pub fn mark_vram_dirty(&mut self) {
        self.render_dirty_flags.mark_vram_dirty();
    }

    pub fn mark_oam_dirty(&mut self) {
        self.render_dirty_flags.mark_oam_dirty();
    }

    pub fn mark_palette_dirty(&mut self) {
        self.render_dirty_flags.mark_palette_dirty();
    }

    pub fn clear_all_dirty_flags(&mut self) {
        self.render_dirty_flags.clear_all();
    }

    pub fn is_any_dirty(&self) -> bool {
        self.render_dirty_flags.is_any_dirty()
    }
}
