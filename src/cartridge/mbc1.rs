use super::{CartridgeInfo, Mbc};

// MBC1 - Memory Bank Controller 1
// Supports up to 512 KiB ROM with up to 32 KiB of banked RAM
#[derive(Debug)]
pub struct Mbc1 {
    info: CartridgeInfo,
    rom_data: Vec<u8>,
    ram_data: Vec<u8>,

    // MBC1 registers
    ram_enabled: bool,
    rom_bank: u8,     // 5-bit register (0x01-0x1F)
    ram_bank: u8,     // 2-bit register (0x00-0x03) or upper ROM bits
    banking_mode: u8, // 0 = ROM banking mode, 1 = RAM banking mode
}

impl Mbc1 {
    pub fn new(rom_data: Vec<u8>, info: CartridgeInfo) -> Result<Self, String> {
        // Calculate RAM size in bytes
        let ram_size_bytes = match info.ram_size {
            0x00 => 0,     // No RAM
            0x02 => 8192,  // 8 KiB (1 bank)
            0x03 => 32768, // 32 KiB (4 banks)
            _ => {
                return Err(format!(
                    "Unsupported RAM size for MBC1: 0x{:02X}",
                    info.ram_size
                ));
            }
        };

        // Initialize RAM with zeros
        let ram_data = vec![0; ram_size_bytes];

        Ok(Mbc1 {
            info,
            rom_data,
            ram_data,
            ram_enabled: false,
            rom_bank: 1, // Default to bank 1
            ram_bank: 0,
            banking_mode: 0, // Default to ROM banking mode
        })
    }

    // Get the effective ROM bank for the 0x0000-0x3FFF region
    fn get_bank_0(&self) -> usize {
        if self.banking_mode == 1 {
            // RAM banking mode: bank 0 can be bank 0, 0x20, 0x40, or 0x60
            ((self.ram_bank as usize) << 5) % self.info.rom_banks
        } else {
            // ROM banking mode: always bank 0
            0
        }
    }

    // Get the effective ROM bank for the 0x4000-0x7FFF region
    fn get_bank_1(&self) -> usize {
        let mut bank = self.rom_bank as usize;

        // Add upper bits from ram_bank register
        bank |= (self.ram_bank as usize) << 5;

        // Handle the zero substitution for combined banks
        // Banks 0x00, 0x20, 0x40, 0x60 become 0x01, 0x21, 0x41, 0x61
        if (bank & 0x1F) == 0 {
            bank += 1;
        }

        // Mask to available banks
        bank % self.info.rom_banks
    }

    // Get the effective RAM bank
    fn get_ram_bank(&self) -> usize {
        if self.banking_mode == 1 && self.info.ram_banks > 1 {
            // RAM banking mode: use ram_bank register
            (self.ram_bank as usize) % self.info.ram_banks
        } else {
            // ROM banking mode or single RAM bank: always use bank 0
            0
        }
    }
}

impl Mbc for Mbc1 {
    fn read(&self, address: u16) -> u8 {
        match address {
            // ROM Bank 00 (0x0000-0x3FFF)
            0x0000..=0x3FFF => {
                let bank = self.get_bank_0();
                let rom_addr = (bank * 0x4000) + (address as usize);

                if rom_addr < self.rom_data.len() {
                    self.rom_data[rom_addr]
                } else {
                    0xFF
                }
            }

            // ROM Bank 01-7F (0x4000-0x7FFF)
            0x4000..=0x7FFF => {
                let bank = self.get_bank_1();
                let rom_addr = (bank * 0x4000) + ((address - 0x4000) as usize);

                if rom_addr < self.rom_data.len() {
                    self.rom_data[rom_addr]
                } else {
                    0xFF
                }
            }

            // External RAM (0xA000-0xBFFF)
            0xA000..=0xBFFF => {
                if !self.ram_enabled || self.ram_data.is_empty() {
                    // Return open bus value when RAM is disabled or not present
                    // Often 0xFF but not guaranteed
                    0xFF
                } else {
                    let ram_bank = self.get_ram_bank();
                    let ram_addr = (ram_bank * 0x2000) + ((address - 0xA000) as usize);

                    if ram_addr < self.ram_data.len() {
                        self.ram_data[ram_addr]
                    } else {
                        0xFF
                    }
                }
            }

            _ => 0xFF,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            // RAM Enable (0x0000-0x1FFF)
            0x0000..=0x1FFF => {
                // RAM is enabled when lower 4 bits equal 0x0A
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }

            // ROM Bank Number (0x2000-0x3FFF)
            0x2000..=0x3FFF => {
                // 5-bit register, values 0x00-0x1F
                // Store the raw value - zero substitution happens in bank calculation
                self.rom_bank = value & 0x1F;
            }

            // RAM Bank Number / Upper ROM Bank bits (0x4000-0x5FFF)
            0x4000..=0x5FFF => {
                // 2-bit register, values 0x00-0x03
                self.ram_bank = value & 0x03;
            }

            // Banking Mode Select (0x6000-0x7FFF)
            0x6000..=0x7FFF => {
                // 1-bit register
                self.banking_mode = value & 0x01;
            }

            // External RAM (0xA000-0xBFFF)
            0xA000..=0xBFFF => {
                if self.ram_enabled && !self.ram_data.is_empty() {
                    let ram_bank = self.get_ram_bank();
                    let ram_addr = (ram_bank * 0x2000) + ((address - 0xA000) as usize);

                    if ram_addr < self.ram_data.len() {
                        self.ram_data[ram_addr] = value;
                    }
                }
                // Writes are ignored when RAM is disabled
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

    fn create_test_mbc1(rom_banks: usize, ram_banks: usize) -> Mbc1 {
        let rom_size = rom_banks * 0x4000; // 16KB per bank
        let rom_data = (0..rom_size).map(|i| i as u8).collect();

        let (ram_size_code, ram_size_banks) = match ram_banks {
            0 => (0x00, 0),
            1 => (0x02, 1),
            4 => (0x03, 4),
            _ => panic!("Invalid RAM bank count"),
        };

        let info = CartridgeInfo {
            cartridge_type: CartridgeType::Mbc1Ram,
            rom_size: 0x04, // 512 KiB
            ram_size: ram_size_code,
            rom_banks,
            ram_banks: ram_size_banks,
        };

        Mbc1::new(rom_data, info).unwrap()
    }

    #[test]
    fn test_mbc1_creation() {
        let mbc1 = create_test_mbc1(32, 4); // 512KB ROM, 32KB RAM
        assert_eq!(mbc1.rom_bank, 1);
        assert_eq!(mbc1.ram_bank, 0);
        assert_eq!(mbc1.banking_mode, 0);
        assert!(!mbc1.ram_enabled);
    }

    #[test]
    fn test_mbc1_ram_enable() {
        let mut mbc1 = create_test_mbc1(4, 1);

        // Initially RAM should be disabled
        assert!(!mbc1.ram_enabled);

        // Enable RAM
        mbc1.write(0x1000, 0x0A);
        assert!(mbc1.ram_enabled);

        // Disable RAM
        mbc1.write(0x1000, 0x00);
        assert!(!mbc1.ram_enabled);
    }

    #[test]
    fn test_mbc1_rom_banking() {
        let mut mbc1 = create_test_mbc1(8, 0);

        // Set ROM bank 2
        mbc1.write(0x2000, 0x02);
        assert_eq!(mbc1.rom_bank, 2);

        // Bank 0 register stores 0, but bank calculation substitutes to 1
        mbc1.write(0x2000, 0x00);
        assert_eq!(mbc1.rom_bank, 0); // Register stores raw value

        // Test upper bits are masked
        mbc1.write(0x2000, 0xFF);
        assert_eq!(mbc1.rom_bank, 0x1F);
    }

    #[test]
    fn test_mbc1_ram_banking() {
        let mut mbc1 = create_test_mbc1(32, 4);

        // Set RAM bank 2
        mbc1.write(0x4000, 0x02);
        assert_eq!(mbc1.ram_bank, 2);

        // Test upper bits are masked
        mbc1.write(0x4000, 0xFF);
        assert_eq!(mbc1.ram_bank, 0x03);
    }

    #[test]
    fn test_mbc1_banking_mode() {
        let mut mbc1 = create_test_mbc1(32, 4);

        // Initially in ROM banking mode
        assert_eq!(mbc1.banking_mode, 0);

        // Switch to RAM banking mode
        mbc1.write(0x6000, 0x01);
        assert_eq!(mbc1.banking_mode, 1);

        // Switch back to ROM banking mode
        mbc1.write(0x6000, 0x00);
        assert_eq!(mbc1.banking_mode, 0);
    }

    #[test]
    fn test_mbc1_read_rom() {
        let mbc1 = create_test_mbc1(4, 0);

        // Read from bank 0 (0x0000-0x3FFF)
        assert_eq!(mbc1.read(0x0000), 0x00);
        assert_eq!(mbc1.read(0x3FFF), 255); // 16383 % 256 = 255

        // Read from bank 1 (0x4000-0x7FFF) - default bank
        // Bank 1 starts at offset 0x4000 in ROM data
        assert_eq!(mbc1.read(0x4000), 0); // 16384 % 256 = 0
        assert_eq!(mbc1.read(0x7FFF), 255); // 32767 % 256 = 255
    }

    #[test]
    fn test_mbc1_read_ram() {
        let mut mbc1 = create_test_mbc1(4, 1);

        // RAM should return 0xFF when disabled
        assert_eq!(mbc1.read(0xA000), 0xFF);

        // Enable RAM
        mbc1.write(0x1000, 0x0A);

        // RAM should return 0x00 (initial value) when enabled
        assert_eq!(mbc1.read(0xA000), 0x00);
    }

    #[test]
    fn test_mbc1_write_ram() {
        let mut mbc1 = create_test_mbc1(4, 1);

        // Enable RAM
        mbc1.write(0x1000, 0x0A);

        // Write to RAM
        mbc1.write(0xA000, 0x42);
        assert_eq!(mbc1.read(0xA000), 0x42);

        // Disable RAM and try to write
        mbc1.write(0x1000, 0x00);
        mbc1.write(0xA000, 0x99);

        // Enable RAM again - value should be unchanged
        mbc1.write(0x1000, 0x0A);
        assert_eq!(mbc1.read(0xA000), 0x42);
    }
}
