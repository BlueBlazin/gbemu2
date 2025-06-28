use super::{CartridgeInfo, Mbc};

// MBC0 - No Memory Bank Controller (ROM only cartridges)
// Simple cartridges with up to 32KB ROM, no banking
#[derive(Debug)]
pub struct Mbc0 {
    info: CartridgeInfo,
    rom_data: Vec<u8>,
}

impl Mbc0 {
    pub fn new(rom_data: Vec<u8>, info: CartridgeInfo) -> Result<Self, String> {
        // Validate that this is indeed a ROM-only cartridge
        if info.rom_banks > 2 {
            return Err("MBC0 cartridges cannot have more than 2 ROM banks (32KB)".to_string());
        }

        if info.ram_banks > 0 {
            return Err("MBC0 cartridges do not support external RAM".to_string());
        }

        // Ensure we have at least 32KB of ROM data
        let mut padded_rom = rom_data;
        if padded_rom.len() < 32768 {
            padded_rom.resize(32768, 0xFF); // Pad with 0xFF (unprogrammed ROM value)
        }

        Ok(Mbc0 {
            info,
            rom_data: padded_rom,
        })
    }
}

impl Mbc for Mbc0 {
    fn read(&self, address: u16) -> u8 {
        match address {
            // ROM Bank 00 (0x0000-0x3FFF) - Fixed bank
            0x0000..=0x3FFF => {
                let rom_addr = address as usize;
                if rom_addr < self.rom_data.len() {
                    self.rom_data[rom_addr]
                } else {
                    0xFF // Return 0xFF for unmapped addresses
                }
            }
            // ROM Bank 01 (0x4000-0x7FFF) - Fixed bank for MBC0
            0x4000..=0x7FFF => {
                let rom_addr = (address - 0x4000) as usize + 0x4000;
                if rom_addr < self.rom_data.len() {
                    self.rom_data[rom_addr]
                } else {
                    0xFF // Return 0xFF for unmapped addresses
                }
            }
            // External RAM (0xA000-0xBFFF) - Not supported in MBC0
            0xA000..=0xBFFF => {
                // MBC0 has no external RAM, return 0xFF
                0xFF
            }
            _ => {
                // Invalid address range for cartridge
                0xFF
            }
        }
    }

    fn write(&mut self, address: u16, _value: u8) {
        match address {
            // ROM areas (0x0000-0x7FFF) - Read-only, writes are ignored
            0x0000..=0x7FFF => {
                // ROM is read-only, ignore writes
                // In some cases, writes to ROM might control banking, but not for MBC0
            }
            // External RAM (0xA000-0xBFFF) - Not supported in MBC0
            0xA000..=0xBFFF => {
                // MBC0 has no external RAM, ignore writes
            }
            _ => {
                // Invalid address range for cartridge, ignore
            }
        }
    }

    fn get_info(&self) -> &CartridgeInfo {
        &self.info
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::CartridgeType;

    #[test]
    fn test_mbc0_creation() {
        let rom_data = vec![0u8; 32768]; // 32KB ROM
        let info = CartridgeInfo {
            cartridge_type: CartridgeType::RomOnly,
            rom_size: 0x00,
            ram_size: 0x00,
            rom_banks: 2,
            ram_banks: 0,
        };

        let mbc0 = Mbc0::new(rom_data, info);
        assert!(mbc0.is_ok());
    }

    #[test]
    fn test_mbc0_read_rom() {
        let mut rom_data = vec![0u8; 32768];
        rom_data[0x0000] = 0x12; // Bank 0
        rom_data[0x4000] = 0x34; // Bank 1

        let info = CartridgeInfo {
            cartridge_type: CartridgeType::RomOnly,
            rom_size: 0x00,
            ram_size: 0x00,
            rom_banks: 2,
            ram_banks: 0,
        };

        let mbc0 = Mbc0::new(rom_data, info).unwrap();

        assert_eq!(mbc0.read(0x0000), 0x12);
        assert_eq!(mbc0.read(0x4000), 0x34);
    }

    #[test]
    fn test_mbc0_read_unmapped() {
        let rom_data = vec![0u8; 16384]; // Only 16KB ROM
        let info = CartridgeInfo {
            cartridge_type: CartridgeType::RomOnly,
            rom_size: 0x00,
            ram_size: 0x00,
            rom_banks: 2,
            ram_banks: 0,
        };

        let mbc0 = Mbc0::new(rom_data, info).unwrap();

        // Should return 0xFF for external RAM (not supported)
        assert_eq!(mbc0.read(0xA000), 0xFF);
        assert_eq!(mbc0.read(0xBFFF), 0xFF);
    }

    #[test]
    fn test_mbc0_write_ignored() {
        let rom_data = vec![0u8; 32768];
        let info = CartridgeInfo {
            cartridge_type: CartridgeType::RomOnly,
            rom_size: 0x00,
            ram_size: 0x00,
            rom_banks: 2,
            ram_banks: 0,
        };

        let mut mbc0 = Mbc0::new(rom_data, info).unwrap();

        // Writes to ROM should be ignored
        mbc0.write(0x0000, 0xFF);
        assert_eq!(mbc0.read(0x0000), 0x00); // Should still be 0

        // Writes to external RAM should be ignored
        mbc0.write(0xA000, 0xFF);
        assert_eq!(mbc0.read(0xA000), 0xFF); // Should return 0xFF (unmapped)
    }
}
