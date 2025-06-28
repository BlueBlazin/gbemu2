#[derive(Debug, Clone)]
pub struct MMU {
    // ROM Banks (0x0000-0x7FFF)
    pub rom_bank_00: [u8; 0x4000], // 0x0000-0x3FFF: Fixed bank
    pub rom_bank_nn: [u8; 0x4000], // 0x4000-0x7FFF: Switchable bank (TODO: implement banking)

    // Video RAM (0x8000-0x9FFF)
    pub vram: [u8; 0x2000], // 8 KiB Video RAM (TODO: connect to PPU)

    // External RAM (0xA000-0xBFFF)
    pub external_ram: [u8; 0x2000], // 8 KiB from cartridge (TODO: implement banking)

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
            rom_bank_00: [0; 0x4000],
            rom_bank_nn: [0; 0x4000],
            vram: [0; 0x2000],
            external_ram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0xA0],
            io_registers: [0; 0x80],
            hram: [0; 0x7F],
            ie_register: 0,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            // ROM Bank 00 (0x0000-0x3FFF)
            0x0000..=0x3FFF => self.rom_bank_00[(address & 0x3FFF) as usize],

            // ROM Bank NN (0x4000-0x7FFF) - TODO: implement banking
            0x4000..=0x7FFF => self.rom_bank_nn[(address & 0x3FFF) as usize],

            // Video RAM (0x8000-0x9FFF) - TODO: connect to PPU
            0x8000..=0x9FFF => self.vram[(address & 0x1FFF) as usize],

            // External RAM (0xA000-0xBFFF) - TODO: implement banking
            0xA000..=0xBFFF => self.external_ram[(address & 0x1FFF) as usize],

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
            // ROM Banks (0x0000-0x7FFF) - TODO: implement mapper control
            0x0000..=0x7FFF => {
                // ROM writes typically control banking, not actual ROM data
                // TODO: implement cartridge mapper handling
            }

            // Video RAM (0x8000-0x9FFF) - TODO: connect to PPU
            0x8000..=0x9FFF => {
                self.vram[(address & 0x1FFF) as usize] = value;
                // TODO: notify PPU of VRAM changes
            }

            // External RAM (0xA000-0xBFFF) - TODO: implement banking
            0xA000..=0xBFFF => {
                self.external_ram[(address & 0x1FFF) as usize] = value;
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

    // Helper method to load ROM data (TODO: extend for proper cartridge handling)
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        // Load first 16KB into bank 00
        let bank_00_size = std::cmp::min(rom_data.len(), 0x4000);
        self.rom_bank_00[..bank_00_size].copy_from_slice(&rom_data[..bank_00_size]);

        // Load next 16KB into bank NN if available
        if rom_data.len() > 0x4000 {
            let bank_nn_size = std::cmp::min(rom_data.len() - 0x4000, 0x4000);
            self.rom_bank_nn[..bank_nn_size]
                .copy_from_slice(&rom_data[0x4000..0x4000 + bank_nn_size]);
        }
    }
}
