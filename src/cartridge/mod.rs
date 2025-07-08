use crate::cartridge::mbc0::Mbc0;
use crate::cartridge::mbc1::Mbc1;
use crate::cartridge::mbc3::Mbc3;

pub mod mbc0;
pub mod mbc1;
pub mod mbc3;

// Cartridge header constants
pub const CARTRIDGE_TYPE_ADDR: u16 = 0x147;
pub const ROM_SIZE_ADDR: u16 = 0x148;
pub const RAM_SIZE_ADDR: u16 = 0x149;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CartridgeType {
    RomOnly = 0x00,
    Mbc1 = 0x01,
    Mbc1Ram = 0x02,
    Mbc1RamBattery = 0x03,
    Mbc3TimerBattery = 0x0F,
    Mbc3TimerRamBattery = 0x10,
    Mbc3 = 0x11,
    Mbc3Ram = 0x12,
    Mbc3RamBattery = 0x13,
    Mbc5 = 0x19,
    Mbc5Ram = 0x1A,
    Mbc5RamBattery = 0x1B,
    Mbc5Rumble = 0x1C,
    Mbc5RumbleRam = 0x1D,
    Mbc5RumbleRamBattery = 0x1E,
    // Add other types as needed
}

impl CartridgeType {
    pub fn from_byte(value: u8) -> Option<Self> {
        match value {
            0x00 | 0x08 | 0x09 => Some(CartridgeType::RomOnly),
            0x01 => Some(CartridgeType::Mbc1),
            0x02 => Some(CartridgeType::Mbc1Ram),
            0x03 => Some(CartridgeType::Mbc1RamBattery),
            0x0F => Some(CartridgeType::Mbc3TimerBattery),
            0x10 => Some(CartridgeType::Mbc3TimerRamBattery),
            0x11 => Some(CartridgeType::Mbc3),
            0x12 => Some(CartridgeType::Mbc3Ram),
            0x13 => Some(CartridgeType::Mbc3RamBattery),
            0x19 => Some(CartridgeType::Mbc5),
            0x1A => Some(CartridgeType::Mbc5Ram),
            0x1B => Some(CartridgeType::Mbc5RamBattery),
            0x1C => Some(CartridgeType::Mbc5Rumble),
            0x1D => Some(CartridgeType::Mbc5RumbleRam),
            0x1E => Some(CartridgeType::Mbc5RumbleRamBattery),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CartridgeInfo {
    pub cartridge_type: CartridgeType,
    pub rom_size: u8,
    pub ram_size: u8,
    pub rom_banks: usize,
    pub ram_banks: usize,
}

impl CartridgeInfo {
    pub fn from_rom(rom_data: &[u8]) -> Result<Self, String> {
        if rom_data.len() < 0x150 {
            return Err("ROM too small to contain valid header".to_string());
        }

        let cartridge_type_byte = rom_data[CARTRIDGE_TYPE_ADDR as usize];
        let cartridge_type = CartridgeType::from_byte(cartridge_type_byte)
            .ok_or_else(|| format!("Unknown cartridge type: 0x{:02X}", cartridge_type_byte))?;

        let rom_size = rom_data[ROM_SIZE_ADDR as usize];
        let ram_size = rom_data[RAM_SIZE_ADDR as usize];

        let rom_banks = match rom_size {
            0x00 => 2,   // 32 KiB (no banking)
            0x01 => 4,   // 64 KiB
            0x02 => 8,   // 128 KiB
            0x03 => 16,  // 256 KiB
            0x04 => 32,  // 512 KiB
            0x05 => 64,  // 1 MiB
            0x06 => 128, // 2 MiB
            0x07 => 256, // 4 MiB
            0x08 => 512, // 8 MiB
            _ => return Err(format!("Invalid ROM size: 0x{:02X}", rom_size)),
        };

        let ram_banks = match ram_size {
            0x00 => 0,  // No RAM
            0x01 => 0,  // Unused
            0x02 => 1,  // 8 KiB (1 bank)
            0x03 => 4,  // 32 KiB (4 banks)
            0x04 => 16, // 128 KiB (16 banks)
            0x05 => 8,  // 64 KiB (8 banks)
            _ => return Err(format!("Invalid RAM size: 0x{:02X}", ram_size)),
        };

        Ok(CartridgeInfo {
            cartridge_type,
            rom_size,
            ram_size,
            rom_banks,
            ram_banks,
        })
    }
}

// Trait for Memory Bank Controllers
pub trait Mbc: std::fmt::Debug {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
    fn get_info(&self) -> &CartridgeInfo;
}

// Main cartridge struct
#[derive(Debug)]
pub struct Cartridge {
    mbc: Box<dyn Mbc>,
}

impl Cartridge {
    pub fn new(rom_data: Vec<u8>) -> Result<Self, String> {
        let info = CartridgeInfo::from_rom(&rom_data)?;

        // Validate ROM size matches expected size
        let expected_rom_size = info.rom_banks * 16384; // 16KB per bank
        if rom_data.len() < expected_rom_size {
            return Err(format!(
                "ROM data size ({} bytes) is smaller than expected ({} bytes)",
                rom_data.len(),
                expected_rom_size
            ));
        }

        let mbc: Box<dyn Mbc> = match info.cartridge_type {
            CartridgeType::RomOnly => Box::new(Mbc0::new(rom_data, info)?),
            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => {
                Box::new(Mbc1::new(rom_data, info)?)
            }
            CartridgeType::Mbc3
            | CartridgeType::Mbc3Ram
            | CartridgeType::Mbc3RamBattery
            | CartridgeType::Mbc3TimerBattery
            | CartridgeType::Mbc3TimerRamBattery => Box::new(Mbc3::new(rom_data, info)),
            _ => {
                return Err(format!(
                    "Unsupported cartridge type: {:?}",
                    info.cartridge_type
                ));
            }
        };

        Ok(Cartridge { mbc })
    }

    pub fn read(&self, address: u16) -> u8 {
        self.mbc.read(address)
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.mbc.write(address, value)
    }

    pub fn get_info(&self) -> &CartridgeInfo {
        self.mbc.get_info()
    }
}

impl Clone for Cartridge {
    fn clone(&self) -> Self {
        // For now, we'll just panic on clone since cloning cartridges with state is complex
        // In a real implementation, you'd need to implement Clone for each MBC type
        panic!("Cartridge cloning not implemented")
    }
}
