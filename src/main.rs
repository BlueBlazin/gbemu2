use clap::Parser;
use gbemu2::emulator::run_emulator;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gbemu2")]
#[command(about = "A cycle-accurate Game Boy (DMG) emulator")]
#[command(version)]
struct Args {
    /// ROM file to load and run
    #[arg(value_name = "ROM_FILE")]
    rom_file: PathBuf,

    /// Print cartridge information and exit
    #[arg(short = 'i', long)]
    info: bool,
}

fn main() {
    let args = Args::parse();

    // Load ROM file
    let rom_data = match fs::read(&args.rom_file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!(
                "Error reading ROM file '{}': {}",
                args.rom_file.display(),
                e
            );
            std::process::exit(1);
        }
    };

    println!(
        "Loaded ROM: {} ({} bytes)",
        args.rom_file.display(),
        rom_data.len()
    );

    // Parse cartridge info
    let cartridge_info = match gbemu2::cartridge::CartridgeInfo::from_rom(&rom_data) {
        Ok(info) => {
            println!("Cartridge Type: {:?}", info.cartridge_type);
            println!("ROM Banks: {} ({} KB)", info.rom_banks, info.rom_banks * 16);
            println!("RAM Banks: {} ({} KB)", info.ram_banks, info.ram_banks * 8);
            info
        }
        Err(e) => {
            eprintln!("Error parsing ROM header: {}", e);
            std::process::exit(1);
        }
    };

    // If info flag is set, just print info and exit
    if args.info {
        return;
    }

    // Check if cartridge type is supported
    match cartridge_info.cartridge_type {
        gbemu2::cartridge::CartridgeType::RomOnly => {
            println!("Starting emulation...");
        }
        unsupported => {
            eprintln!("Unsupported cartridge type: {:?}", unsupported);
            eprintln!("Currently only ROM-only cartridges (MBC0) are supported.");
            std::process::exit(1);
        }
    }

    // Run emulator with the ROM data
    run_emulator(Some(rom_data));
}
