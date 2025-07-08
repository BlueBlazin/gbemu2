use super::{CartridgeInfo, Mbc};

#[derive(Debug)]
pub struct Mbc3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rtc_registers: [u8; 5], // RTC S, M, H, DL, DH
    rtc_latched: [u8; 5],   // Latched RTC values
    info: CartridgeInfo,
    ram_rtc_enabled: bool,
    rom_bank: u8,
    ram_rtc_select: u8,
    latch_clock_data: u8,
}

impl Mbc3 {
    pub fn new(rom: Vec<u8>, info: CartridgeInfo) -> Self {
        let ram_size = info.ram_banks * 8 * 1024;
        let ram = vec![0; ram_size];

        Self {
            rom,
            ram,
            rtc_registers: [0; 5],
            rtc_latched: [0; 5],
            info,
            ram_rtc_enabled: false,
            rom_bank: 1,
            ram_rtc_select: 0,
            latch_clock_data: 0xFF,
        }
    }

    fn get_rom_bank_0(&self) -> usize {
        // ROM bank 0 is always fixed at 0x0000-0x3FFF
        0
    }

    fn get_rom_bank_n(&self) -> usize {
        // ROM bank for 0x4000-0x7FFF
        // MBC3 supports up to 128 banks (7-bit)
        let bank = if self.rom_bank == 0 { 1 } else { self.rom_bank };
        (bank as usize) % self.info.rom_banks
    }

    fn get_ram_bank(&self) -> usize {
        // Only use lower 3 bits for RAM bank selection (0-7)
        (self.ram_rtc_select as usize & 0x07) % self.info.ram_banks.max(1)
    }

    fn is_rtc_register(&self) -> bool {
        // RTC registers are selected with values 0x08-0x0C
        (0x08..=0x0C).contains(&self.ram_rtc_select)
    }

    fn get_rtc_register_index(&self) -> usize {
        // Map 0x08-0x0C to array indices 0-4
        (self.ram_rtc_select - 0x08) as usize
    }
}

impl Mbc for Mbc3 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                // ROM Bank 0
                let bank = self.get_rom_bank_0();
                let rom_address = bank * 0x4000 + (address as usize);
                self.rom.get(rom_address).copied().unwrap_or(0xFF)
            }
            0x4000..=0x7FFF => {
                // ROM Bank N (1-127)
                let bank = self.get_rom_bank_n();
                let rom_address = bank * 0x4000 + ((address - 0x4000) as usize);
                self.rom.get(rom_address).copied().unwrap_or(0xFF)
            }
            0xA000..=0xBFFF => {
                // RAM Bank or RTC Register
                if !self.ram_rtc_enabled {
                    return 0xFF;
                }

                if self.is_rtc_register() {
                    // Read RTC register
                    let index = self.get_rtc_register_index();
                    self.rtc_latched.get(index).copied().unwrap_or(0xFF)
                } else if !self.ram.is_empty() {
                    // Read RAM bank
                    let bank = self.get_ram_bank();
                    let ram_address = bank * 0x2000 + ((address - 0xA000) as usize);
                    self.ram.get(ram_address).copied().unwrap_or(0xFF)
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                // RAM and Timer Enable
                self.ram_rtc_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                // ROM Bank Number (7-bit)
                self.rom_bank = value & 0x7F;
            }
            0x4000..=0x5FFF => {
                // RAM Bank Number or RTC Register Select
                self.ram_rtc_select = value & 0x0F;
            }
            0x6000..=0x7FFF => {
                // Latch Clock Data
                let prev_latch = self.latch_clock_data;
                self.latch_clock_data = value;

                // Latch RTC on transition from 0x00 to 0x01
                if prev_latch == 0x00 && value == 0x01 {
                    // Copy current RTC registers to latched values
                    self.rtc_latched.copy_from_slice(&self.rtc_registers);
                }
            }
            0xA000..=0xBFFF => {
                // RAM Bank or RTC Register Write
                if !self.ram_rtc_enabled {
                    return;
                }

                if self.is_rtc_register() {
                    // Write to RTC register
                    let index = self.get_rtc_register_index();
                    if index < self.rtc_registers.len() {
                        self.rtc_registers[index] = value;
                    }
                } else if !self.ram.is_empty() {
                    // Write to RAM bank
                    let bank = self.get_ram_bank();
                    let ram_address = bank * 0x2000 + ((address - 0xA000) as usize);
                    if let Some(byte) = self.ram.get_mut(ram_address) {
                        *byte = value;
                    }
                }
            }
            _ => {}
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

    fn create_test_mbc3(rom_banks: usize, ram_banks: usize) -> Mbc3 {
        let rom_size = rom_banks * 16 * 1024;
        let mut rom = vec![0; rom_size];

        // Fill ROM with test pattern
        for (i, byte) in rom.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }

        let info = CartridgeInfo {
            cartridge_type: CartridgeType::Mbc3,
            rom_size: 0, // Not used in calculation
            ram_size: 0, // Not used in calculation
            rom_banks,
            ram_banks,
        };

        Mbc3::new(rom, info)
    }

    #[test]
    fn test_mbc3_creation() {
        let mbc3 = create_test_mbc3(4, 1);
        assert_eq!(mbc3.info.rom_banks, 4);
        assert_eq!(mbc3.info.ram_banks, 1);
        assert!(!mbc3.ram_rtc_enabled);
        assert_eq!(mbc3.rom_bank, 1);
        assert_eq!(mbc3.ram_rtc_select, 0);
    }

    #[test]
    fn test_rom_banking() {
        let mut mbc3 = create_test_mbc3(8, 0);

        // Default bank 1
        assert_eq!(mbc3.read(0x4000), 0x00); // First byte of bank 1

        // Switch to bank 2
        mbc3.write(0x2000, 0x02);
        assert_eq!(mbc3.read(0x4000), 0x00); // First byte of bank 2

        // Bank 0 should map to bank 1
        mbc3.write(0x2000, 0x00);
        assert_eq!(mbc3.read(0x4000), 0x00); // First byte of bank 1

        // Test 7-bit masking
        mbc3.write(0x2000, 0xFF);
        assert_eq!(mbc3.rom_bank, 0x7F);
    }

    #[test]
    fn test_ram_enable() {
        let mut mbc3 = create_test_mbc3(2, 1);

        // RAM disabled by default
        assert_eq!(mbc3.read(0xA000), 0xFF);
        mbc3.write(0xA000, 0x42);
        assert_eq!(mbc3.read(0xA000), 0xFF);

        // Enable RAM
        mbc3.write(0x0000, 0x0A);
        mbc3.write(0xA000, 0x42);
        assert_eq!(mbc3.read(0xA000), 0x42);

        // Disable RAM
        mbc3.write(0x0000, 0x00);
        assert_eq!(mbc3.read(0xA000), 0xFF);
    }

    #[test]
    fn test_ram_banking() {
        let mut mbc3 = create_test_mbc3(2, 4);
        mbc3.write(0x0000, 0x0A); // Enable RAM

        // Write to different banks
        mbc3.write(0x4000, 0x00); // Bank 0
        mbc3.write(0xA000, 0x11);

        mbc3.write(0x4000, 0x01); // Bank 1
        mbc3.write(0xA000, 0x22);

        mbc3.write(0x4000, 0x02); // Bank 2
        mbc3.write(0xA000, 0x33);

        mbc3.write(0x4000, 0x03); // Bank 3
        mbc3.write(0xA000, 0x44);

        // Read back from different banks
        mbc3.write(0x4000, 0x00);
        assert_eq!(mbc3.read(0xA000), 0x11);

        mbc3.write(0x4000, 0x01);
        assert_eq!(mbc3.read(0xA000), 0x22);

        mbc3.write(0x4000, 0x02);
        assert_eq!(mbc3.read(0xA000), 0x33);

        mbc3.write(0x4000, 0x03);
        assert_eq!(mbc3.read(0xA000), 0x44);
    }

    #[test]
    fn test_rtc_registers() {
        let mut mbc3 = create_test_mbc3(2, 1);
        mbc3.write(0x0000, 0x0A); // Enable RTC

        // Write to RTC registers
        mbc3.write(0x4000, 0x08); // Select RTC S
        mbc3.write(0xA000, 0x3B); // 59 seconds

        mbc3.write(0x4000, 0x09); // Select RTC M
        mbc3.write(0xA000, 0x3B); // 59 minutes

        mbc3.write(0x4000, 0x0A); // Select RTC H
        mbc3.write(0xA000, 0x17); // 23 hours

        mbc3.write(0x4000, 0x0B); // Select RTC DL
        mbc3.write(0xA000, 0xFF); // Day counter low

        mbc3.write(0x4000, 0x0C); // Select RTC DH
        mbc3.write(0xA000, 0xC1); // Day counter high + flags

        // Latch the RTC
        mbc3.write(0x6000, 0x00);
        mbc3.write(0x6000, 0x01);

        // Read latched values
        mbc3.write(0x4000, 0x08);
        assert_eq!(mbc3.read(0xA000), 0x3B);

        mbc3.write(0x4000, 0x09);
        assert_eq!(mbc3.read(0xA000), 0x3B);

        mbc3.write(0x4000, 0x0A);
        assert_eq!(mbc3.read(0xA000), 0x17);

        mbc3.write(0x4000, 0x0B);
        assert_eq!(mbc3.read(0xA000), 0xFF);

        mbc3.write(0x4000, 0x0C);
        assert_eq!(mbc3.read(0xA000), 0xC1);
    }

    #[test]
    fn test_rtc_latch_mechanism() {
        let mut mbc3 = create_test_mbc3(2, 1);
        mbc3.write(0x0000, 0x0A); // Enable RTC

        // Set initial RTC value
        mbc3.write(0x4000, 0x08); // Select RTC S
        mbc3.write(0xA000, 0x10);

        // Latch the value
        mbc3.write(0x6000, 0x00);
        mbc3.write(0x6000, 0x01);

        // Verify latched value
        assert_eq!(mbc3.read(0xA000), 0x10);

        // Change the live register
        mbc3.write(0xA000, 0x20);

        // Latched value should remain unchanged
        assert_eq!(mbc3.read(0xA000), 0x10);

        // Re-latch to get new value
        mbc3.write(0x6000, 0x00);
        mbc3.write(0x6000, 0x01);
        assert_eq!(mbc3.read(0xA000), 0x20);
    }

    #[test]
    fn test_ram_rtc_select_masking() {
        let mut mbc3 = create_test_mbc3(2, 4);

        // Test that only lower 4 bits are used
        mbc3.write(0x4000, 0xFF);
        assert_eq!(mbc3.ram_rtc_select, 0x0F);

        // Verify RTC register detection
        mbc3.write(0x4000, 0x07);
        assert!(!mbc3.is_rtc_register());

        mbc3.write(0x4000, 0x08);
        assert!(mbc3.is_rtc_register());

        mbc3.write(0x4000, 0x0C);
        assert!(mbc3.is_rtc_register());

        mbc3.write(0x4000, 0x0D);
        assert!(!mbc3.is_rtc_register());
    }
}
