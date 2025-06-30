use crate::cartridge::{CartridgeInfo, CartridgeType};
use std::fs;
use std::path::Path;

pub struct RomLoader {
    pub rom_data: Vec<u8>,
    pub cartridge_info: CartridgeInfo,
}

impl RomLoader {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();

        // Load ROM file
        let rom_data = fs::read(path)
            .map_err(|e| format!("Error reading ROM file '{}': {}", path.display(), e))?;

        println!("Loaded ROM: {} ({} bytes)", path.display(), rom_data.len());

        // Parse cartridge info
        let cartridge_info = CartridgeInfo::from_rom(&rom_data)?;

        println!("Cartridge Type: {:?}", cartridge_info.cartridge_type);
        println!(
            "ROM Banks: {} ({} KB)",
            cartridge_info.rom_banks,
            cartridge_info.rom_banks * 16
        );
        println!(
            "RAM Banks: {} ({} KB)",
            cartridge_info.ram_banks,
            cartridge_info.ram_banks * 8
        );

        Ok(RomLoader {
            rom_data,
            cartridge_info,
        })
    }

    pub fn verify_supported(&self) -> Result<(), String> {
        match self.cartridge_info.cartridge_type {
            CartridgeType::RomOnly
            | CartridgeType::Mbc1
            | CartridgeType::Mbc1Ram
            | CartridgeType::Mbc1RamBattery => Ok(()),
            unsupported => Err(format!(
                "Unsupported cartridge type: {:?}. Currently only ROM-only (MBC0) and MBC1 cartridges are supported.",
                unsupported
            )),
        }
    }
}
