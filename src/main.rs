use clap::Parser;
use gbemu2::driver::Event;
use gbemu2::emulator::{create_driver, init_emulator, load_cartridge, with_state};
use sdl2::event::Event as SdlEvent;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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

    /// Scale factor for the display (default: 4)
    #[arg(short = 's', long, default_value = "4")]
    scale: u32,
}

const GB_WIDTH: u32 = 160;
const GB_HEIGHT: u32 = 144;
const GB_CLOCK_SPEED: f64 = 4_194_304.0; // Hz
const CYCLES_PER_FRAME: usize = (GB_CLOCK_SPEED / 60.0) as usize; // ~69905 cycles

// Game Boy DMG color palette (greenish tint)
const COLORS: [(u8, u8, u8); 4] = [
    (155, 188, 15), // Color 0 (lightest)
    (139, 172, 15), // Color 1
    (48, 98, 48),   // Color 2
    (15, 56, 15),   // Color 3 (darkest)
];

fn main() -> Result<(), String> {
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
        return Ok(());
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

    // Initialize SDL2
    let sdl_context = sdl2::init().map_err(|e| e.to_string())?;
    let video_subsystem = sdl_context.video().map_err(|e| e.to_string())?;

    let window_width = GB_WIDTH * args.scale;
    let window_height = GB_HEIGHT * args.scale;

    let window = video_subsystem
        .window("GBEmu2", window_width, window_height)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    // Create texture for Game Boy screen
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, GB_WIDTH, GB_HEIGHT)
        .map_err(|e| e.to_string())?;

    // Initialize emulator
    init_emulator();
    load_cartridge(rom_data).map_err(|e| format!("Failed to load cartridge: {}", e))?;

    let mut driver = create_driver();
    let mut event_pump = sdl_context.event_pump().map_err(|e| e.to_string())?;

    // Timing variables
    let target_frame_duration = Duration::from_secs_f64(1.0 / 60.0);
    let mut last_fps_time = Instant::now();
    let mut last_debug_time = Instant::now();

    println!("Emulator started. Press ESC to quit.");
    println!("Note: CPU implementation is incomplete, so execution may be limited.");

    // Initial render to show boot state
    render_frame(&mut texture, &mut canvas, args.scale)?;

    let mut frame_count = 0u64;
    let mut total_cycles = 0usize;
    let mut fps_frame_count = 0u32;

    'running: loop {
        let frame_start = Instant::now();

        // Handle SDL events
        for event in event_pump.poll_iter() {
            match event {
                SdlEvent::Quit { .. } => break 'running,
                SdlEvent::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // Run emulator until we get a VBlank event
        let mut vblank_occurred = false;
        let mut frame_cycles = 0usize;
        
        // Keep running until we get a VBlank
        while !vblank_occurred {
            // Run for a batch of cycles
            let result = driver.run_for(10000); // Run for 10k cycles at a time
            frame_cycles += result.cycles_executed;
            total_cycles += result.cycles_executed;
            
            match result.event {
                Event::VBlank => {
                    // Render the frame
                    render_frame(&mut texture, &mut canvas, args.scale)?;
                    vblank_occurred = true;
                    frame_count += 1;
                    fps_frame_count += 1;
                }
                Event::AudioBufferFull => {
                    // TODO: Handle audio when APU is implemented
                }
                Event::MaxCycles => {
                    // Continue running
                }
            }
            
            // Safety check: if no cycles were executed, something is wrong
            if result.cycles_executed == 0 {
                println!("Warning: No cycles executed. Emulator may be stuck.");
                println!("Debug: frame_cycles = {}, total_cycles = {}", frame_cycles, total_cycles);
                // Try to render current state
                render_frame(&mut texture, &mut canvas, args.scale)?;
                // Give it one more chance with a smaller batch
                let retry_result = driver.run_for(100);
                if retry_result.cycles_executed == 0 {
                    println!("Emulator appears to be stuck. Exiting.");
                    break 'running;
                }
            }
            
            // Additional safety: if we've run way too many cycles without VBlank, something is wrong
            if frame_cycles > CYCLES_PER_FRAME * 2 {
                println!("Warning: Ran {} cycles without VBlank (expected ~{})", frame_cycles, CYCLES_PER_FRAME);
                render_frame(&mut texture, &mut canvas, args.scale)?;
                break; // Exit inner loop to maintain frame timing
            }
        }

        // Frame timing
        let frame_duration = frame_start.elapsed();
        if frame_duration < target_frame_duration {
            std::thread::sleep(target_frame_duration - frame_duration);
        }

        // Print FPS every second
        let now = Instant::now();
        if now.duration_since(last_fps_time) >= Duration::from_secs(1) {
            let fps = fps_frame_count as f64;
            println!("FPS: {:.1}, Total frames: {}, Total cycles: {}", fps, frame_count, total_cycles);
            last_fps_time = now;
            fps_frame_count = 0;
        }
        
        // Debug output every few seconds
        if now.duration_since(last_debug_time) >= Duration::from_secs(5) {
            println!("Debug: Frame cycles: {}, VBlank: {}", frame_cycles, vblank_occurred);
            // Print PPU state
            let (ly, mode, lcdc) = with_state(|state| {
                (state.ppu.ly, state.ppu.ppu_mode, state.ppu.lcdc)
            });
            println!("Debug: PPU LY={}, Mode={}, LCDC=0x{:02X}", ly, mode, lcdc);
            last_debug_time = now;
        }
    }

    Ok(())
}

fn render_frame(
    texture: &mut sdl2::render::Texture,
    canvas: &mut sdl2::render::WindowCanvas,
    scale: u32,
) -> Result<(), String> {
    // Get framebuffer from PPU
    let framebuffer = with_state(|state| {
        let fb = state.ppu.get_framebuffer();
        let mut buffer = [[0u8; 160]; 144];
        for y in 0..144 {
            for x in 0..160 {
                buffer[y][x] = fb[y][x];
            }
        }
        buffer
    });

    // Convert Game Boy 2-bit colors to RGB
    texture
        .with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for y in 0..GB_HEIGHT as usize {
                for x in 0..GB_WIDTH as usize {
                    let color_index = framebuffer[y][x] as usize;
                    let (r, g, b) = COLORS[color_index];

                    let offset = y * pitch + x * 3;
                    buffer[offset] = r;
                    buffer[offset + 1] = g;
                    buffer[offset + 2] = b;
                }
            }
        })
        .map_err(|e| e.to_string())?;

    // Clear canvas and render texture
    canvas.clear();
    canvas
        .copy(
            &texture,
            None,
            Some(Rect::new(0, 0, GB_WIDTH * scale, GB_HEIGHT * scale)),
        )
        .map_err(|e| e.to_string())?;
    canvas.present();

    Ok(())
}