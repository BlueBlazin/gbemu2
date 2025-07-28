use crate::cartridge::Cartridge;
use crate::ppu::PpuState;
use crate::state::RenderDirtyFlags;

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct MMU {
    // Cartridge (handles ROM and external RAM)
    pub cartridge: Option<Cartridge>,

    // Work RAM (0xC000-0xDFFF)
    pub wram: [u8; 0x2000], // 8 KiB Work RAM

    // I/O Registers (0xFF00-0xFF7F)
    pub io_registers: [u8; 0x80], // 128 bytes I/O (handled by io module)

    // High RAM (0xFF80-0xFFFE)
    pub hram: [u8; 0x7F], // 127 bytes High RAM

    // Interrupt Enable Register (0xFFFF)
    pub ie_register: u8, // Interrupt Enable (handled by io module)
}

impl Default for MMU {
    fn default() -> Self {
        Self::new()
    }
}

impl MMU {
    pub const fn new() -> Self {
        let mut io_registers = [0xFF; 0x80];

        // Set boot values for I/O registers
        io_registers[0x00] = 0xCF; // P1 (FF00) = $CF
        io_registers[0x01] = 0x00; // SB (FF01) = $00
        io_registers[0x02] = 0x7E; // SC (FF02) = $7E

        Self {
            cartridge: None,
            wram: [0xFF; 0x2000],
            io_registers,
            hram: [0xFF; 0x7F],
            ie_register: 0,
        }
    }

    pub fn read(&self, address: u16, dma_active: bool, ppu: &PpuState) -> u8 {
        if dma_active {
            // During DMA, CPU can only access HRAM (0xFF80-0xFFFE)
            match address {
                0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
                _ => 0xFF, // All other addresses return 0xFF during DMA
            }
        } else {
            match address {
                0x8000..=0x9FFF if ppu.ppu_mode == 3 => 0xFF,
                _ => self.read_unrestricted(address, ppu),
            }
        }
    }

    pub fn read_unrestricted(&self, address: u16, ppu: &PpuState) -> u8 {
        match address {
            // ROM and External RAM (0x0000-0x7FFF, 0xA000-0xBFFF) - handled by cartridge
            0x0000..=0x7FFF | 0xA000..=0xBFFF => {
                if let Some(ref cartridge) = self.cartridge {
                    cartridge.read(address)
                } else {
                    0xFF // No cartridge loaded
                }
            }

            // Video RAM (0x8000-0x9FFF) - delegated to PPU
            0x8000..=0x9FFF => self.read_vram(address, ppu),

            // Work RAM (0xC000-0xDFFF)
            0xC000..=0xDFFF => self.wram[(address & 0x1FFF) as usize],

            // Echo RAM (0xE000-0xFDFF) - mirror of 0xC000-0xDDFF
            0xE000..=0xFDFF => {
                let mirrored_addr = address - 0x2000;
                self.wram[(mirrored_addr & 0x1FFF) as usize]
            }

            // Object Attribute Memory (0xFE00-0xFE9F) - delegated to PPU with access restrictions
            0xFE00..=0xFE9F => {
                // Check if OAM is accessible based on PPU mode
                let mode = ppu.ppu_mode;
                if mode == 2 || mode == 3 {
                    // Mode 2 (OAM Scan) or Mode 3 (Drawing) - OAM not accessible
                    0xFF
                } else {
                    // Mode 0 (HBlank) or Mode 1 (VBlank) - OAM accessible
                    self.read_oam(address, ppu)
                }
            }

            // Not Usable (0xFEA0-0xFEFF) - return 0xFF
            0xFEA0..=0xFEFF => 0xFF,

            // I/O Registers (0xFF00-0xFF7F)
            0xFF00..=0xFF7F => {
                // // I/O registers should be handled by EmulatorState
                // // Return default value if we somehow get here
                // self.io_registers[(address - 0xFF00) as usize]
                panic!("IO register accessed from wrong place.");
            }

            // High RAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],

            // Interrupt Enable Register (0xFFFF)
            0xFFFF => self.ie_register,
        }
    }

    pub fn read_vram(&self, address: u16, ppu: &PpuState) -> u8 {
        let vram_addr = (address & 0x1FFF) as usize;
        if vram_addr < 0x1800 {
            // Tile data area (0x8000-0x97FF)
            let tile_index = vram_addr / 16;
            let byte_in_tile = vram_addr % 16;
            ppu.vram[tile_index][byte_in_tile]
        } else {
            // Tile map areas (0x9800-0x9FFF)
            let map_addr = vram_addr - 0x1800;
            if map_addr < 0x400 {
                ppu.bg_tile_map_0[map_addr]
            } else {
                ppu.bg_tile_map_1[map_addr - 0x400]
            }
        }
    }

    pub fn read_oam(&self, address: u16, ppu: &PpuState) -> u8 {
        // Mode 0 (HBlank) or Mode 1 (VBlank) - OAM accessible
        let oam_index = ((address - 0xFE00) / 4) as usize;
        let byte_offset = (address - 0xFE00) % 4;
        if oam_index < 40 {
            match byte_offset {
                0 => ppu.oam[oam_index].y,
                1 => ppu.oam[oam_index].x,
                2 => ppu.oam[oam_index].tile_index,
                3 => ppu.oam[oam_index].attributes,
                _ => 0xFF,
            }
        } else {
            0xFF
        }
    }

    pub fn write(
        &mut self,
        address: u16,
        value: u8,
        dma_active: bool,
        ppu: &mut PpuState,
        dirty_flags: &mut RenderDirtyFlags,
    ) {
        if dma_active {
            // During DMA, CPU can only access HRAM (0xFF80-0xFFFE)
            match address {
                0xFF80..=0xFFFE => {
                    self.hram[(address - 0xFF80) as usize] = value;
                }
                _ => {
                    // All other writes are ignored during DMA
                }
            }
        } else {
            match address {
                // Write to VRAM not allowed during mode 3
                0x8000..=0x9FFF if ppu.ppu_mode == 3 => (),
                _ => self.write_unrestricted(address, value, ppu, dirty_flags),
            }
        }
    }

    pub fn write_unrestricted(
        &mut self,
        address: u16,
        value: u8,
        ppu: &mut PpuState,
        dirty_flags: &mut RenderDirtyFlags,
    ) {
        match address {
            // ROM and External RAM (0x0000-0x7FFF, 0xA000-0xBFFF) - handled by cartridge
            0x0000..=0x7FFF | 0xA000..=0xBFFF => {
                if let Some(ref mut cartridge) = self.cartridge {
                    cartridge.write(address, value);
                }
                // If no cartridge loaded, ignore writes
            }

            // Video RAM (0x8000-0x9FFF) - delegated to PPU
            0x8000..=0x9FFF => {
                let vram_addr = (address & 0x1FFF) as usize;
                if vram_addr < 0x1800 {
                    // Tile data area (0x8000-0x97FF)
                    let tile_index = vram_addr / 16;
                    let byte_in_tile = vram_addr % 16;
                    ppu.vram[tile_index][byte_in_tile] = value;
                    // Mark tile data as dirty
                    dirty_flags.tile_data = true;
                } else {
                    // Tile map areas (0x9800-0x9FFF)
                    let map_addr = vram_addr - 0x1800;
                    if map_addr < 0x400 {
                        ppu.bg_tile_map_0[map_addr] = value;
                        // Mark tilemap 0 as dirty
                        dirty_flags.bg_tilemap_0 = true;
                    } else {
                        ppu.bg_tile_map_1[map_addr - 0x400] = value;
                        // Mark tilemap 1 as dirty
                        dirty_flags.bg_tilemap_1 = true;
                    }
                }
            }

            // Work RAM (0xC000-0xDFFF)
            0xC000..=0xDFFF => {
                self.wram[(address & 0x1FFF) as usize] = value;
            }

            // Echo RAM (0xE000-0xFDFF) - mirror of 0xC000-0xDDFF
            0xE000..=0xFDFF => {
                let mirrored_addr = address - 0x2000;
                self.wram[(mirrored_addr & 0x1FFF) as usize] = value;
            }

            // Object Attribute Memory (0xFE00-0xFE9F) - delegated to PPU with access restrictions
            0xFE00..=0xFE9F => {
                // Check if OAM is accessible based on PPU mode
                let mode = ppu.ppu_mode;
                if mode != 2 && mode != 3 {
                    // OAM accessible in modes 0, 1 but not modes 2, 3
                    let oam_index = ((address - 0xFE00) / 4) as usize;
                    let byte_offset = (address - 0xFE00) % 4;
                    if oam_index < 40 {
                        match byte_offset {
                            0 => ppu.oam[oam_index].y = value,
                            1 => ppu.oam[oam_index].x = value,
                            2 => ppu.oam[oam_index].tile_index = value,
                            3 => ppu.oam[oam_index].attributes = value,
                            _ => {}
                        }
                        // Mark OAM/sprite data as dirty
                        dirty_flags.sprite_tiles = true;
                    }
                }
                // Writes are ignored when OAM is not accessible
            }

            // Not Usable (0xFEA0-0xFEFF) - ignore writes
            0xFEA0..=0xFEFF => {}

            // I/O Registers (0xFF00-0xFF7F)
            0xFF00..=0xFF7F => {
                // // I/O registers should be handled by EmulatorState
                // // Store in local array if we somehow get here
                // self.io_registers[(address - 0xFF00) as usize] = value;
                panic!("IO register accessed from wrong place.");
            }

            // High RAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => {
                self.hram[(address - 0xFF80) as usize] = value;
            }

            // Interrupt Enable Register (0xFFFF)
            0xFFFF => {
                self.ie_register = value;
            }
        }
    }

    // Load a cartridge into the MMU
    pub fn load_cartridge(&mut self, rom_data: Vec<u8>) -> Result<(), String> {
        let cartridge = Cartridge::new(rom_data)?;
        self.cartridge = Some(cartridge);
        Ok(())
    }

    // Remove the cartridge
    pub fn remove_cartridge(&mut self) {
        self.cartridge = None;
    }

    // Get cartridge info if loaded
    pub fn get_cartridge_info(&self) -> Option<&crate::cartridge::CartridgeInfo> {
        self.cartridge.as_ref().map(|cart| cart.get_info())
    }
}
