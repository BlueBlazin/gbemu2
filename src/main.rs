use clap::Parser;
use gbemu2::emulation_runner::EmulationRunner;
use gbemu2::emulator::{create_driver, init_emulator_with_debug, load_cartridge};
use gbemu2::renderer::{Renderer, SdlSystem};
use gbemu2::rom_loader::RomLoader;
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

    /// Scale factor for the display (default: 3)
    #[arg(short = 's', long, default_value = "3")]
    scale: u32,

    /// Debug mode: save a screenshot after 5 seconds
    #[arg(short = 'd', long)]
    debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load and verify ROM
    let rom_loader = RomLoader::load(&args.rom_file)?;

    // If info flag is set, just print info and exit
    if args.info {
        return Ok(());
    }

    // Verify the cartridge is supported
    rom_loader.verify_supported()?;

    println!("Starting emulation...");

    // Initialize emulator with debug mode if requested
    init_emulator_with_debug(args.debug);
    load_cartridge(rom_loader.rom_data)?;

    // Initialize SDL system and create renderer
    let (mut sdl_system, sdl_context) = SdlSystem::new()?;
    let rom_filename = args.rom_file.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown ROM");
    let renderer = Renderer::new(args.scale, &sdl_context, rom_filename)?;

    // Create driver and runner
    let driver = create_driver();
    let mut runner = EmulationRunner::new(driver, renderer);

    // Run the emulation with debug flag
    runner.run(|| sdl_system.poll_events(), args.debug)?;

    Ok(())
}
