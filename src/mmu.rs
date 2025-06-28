use crate::cartridge::Cartridge;

#[derive(Debug)]
pub struct MMU {
    // Cartridge (handles ROM and external RAM)
    pub cartridge: Option<Cartridge>,

    // Video RAM (0x8000-0x9FFF)
    pub vram: [u8; 0x2000], // 8 KiB Video RAM (TODO: connect to PPU)

    // Work RAM (0xC000-0xDFFF)
    pub wram: [u8; 0x2000], // 8 KiB Work RAM

    // Object Attribute Memory (0xFE00-0xFE9F)
    pub oam: [u8; 0xA0], // 160 bytes OAM (TODO: connect to PPU)

    // I/O Registers (0xFF00-0xFF7F)
    pub io_registers: [u8; 0x80], // 128 bytes I/O (TODO: implement proper I/O handling)

    // High RAM (0xFF80-0xFFFE)
    pub hram: [u8; 0x7F], // 127 bytes High RAM

    // Interrupt Enable Register (0xFFFF)
    pub ie_register: u8, // Interrupt Enable (TODO: implement interrupts)
}

impl MMU {
    pub fn new() -> Self {
        Self {
            cartridge: None,
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0xA0],
            io_registers: [0; 0x80],
            hram: [0; 0x7F],
            ie_register: 0,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            // ROM and External RAM (0x0000-0x7FFF, 0xA000-0xBFFF) - handled by cartridge
            0x0000..=0x7FFF | 0xA000..=0xBFFF => {
                if let Some(ref cartridge) = self.cartridge {
                    cartridge.read(address)
                } else {
                    0xFF // No cartridge loaded
                }
            }

            // Video RAM (0x8000-0x9FFF) - TODO: connect to PPU
            0x8000..=0x9FFF => self.vram[(address & 0x1FFF) as usize],

            // Work RAM (0xC000-0xDFFF)
            0xC000..=0xDFFF => self.wram[(address & 0x1FFF) as usize],

            // Echo RAM (0xE000-0xFDFF) - mirror of 0xC000-0xDDFF
            0xE000..=0xFDFF => {
                let mirrored_addr = address - 0x2000;
                self.wram[(mirrored_addr & 0x1FFF) as usize]
            }

            // Object Attribute Memory (0xFE00-0xFE9F) - TODO: connect to PPU
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],

            // Not Usable (0xFEA0-0xFEFF) - return 0xFF
            0xFEA0..=0xFEFF => 0xFF,

            // I/O Registers (0xFF00-0xFF7F)
            0xFF00..=0xFF7F => {
                // Use I/O register system for proper handling
                crate::io::read_io_register(address)
            }

            // High RAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],

            // Interrupt Enable Register (0xFFFF)
            0xFFFF => crate::io::read_io_register(address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            // ROM and External RAM (0x0000-0x7FFF, 0xA000-0xBFFF) - handled by cartridge
            0x0000..=0x7FFF | 0xA000..=0xBFFF => {
                if let Some(ref mut cartridge) = self.cartridge {
                    cartridge.write(address, value);
                }
                // If no cartridge loaded, ignore writes
            }

            // Video RAM (0x8000-0x9FFF) - TODO: connect to PPU
            0x8000..=0x9FFF => {
                self.vram[(address & 0x1FFF) as usize] = value;
                // TODO: notify PPU of VRAM changes
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

            // Object Attribute Memory (0xFE00-0xFE9F) - TODO: connect to PPU
            0xFE00..=0xFE9F => {
                self.oam[(address - 0xFE00) as usize] = value;
                // TODO: notify PPU of OAM changes
            }

            // Not Usable (0xFEA0-0xFEFF) - ignore writes
            0xFEA0..=0xFEFF => {}

            // I/O Registers (0xFF00-0xFF7F)
            0xFF00..=0xFF7F => {
                // Use I/O register system for proper handling
                crate::io::write_io_register(address, value);
            }

            // High RAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => {
                self.hram[(address - 0xFF80) as usize] = value;
            }

            // Interrupt Enable Register (0xFFFF)
            0xFFFF => {
                crate::io::write_io_register(address, value);
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
