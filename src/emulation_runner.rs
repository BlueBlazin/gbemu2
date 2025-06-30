use crate::driver::{Driver, Event};
use crate::emulator::with_state;
use crate::renderer::Renderer;
use image::{ImageBuffer, Rgb};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const GB_CLOCK_SPEED: f64 = 4_194_304.0; // Hz
pub const CYCLES_PER_FRAME: usize = (GB_CLOCK_SPEED / 60.0) as usize; // ~69905 cycles

pub struct EmulationRunner {
    driver: Driver,
    renderer: Renderer,
    target_frame_duration: Duration,
}

pub struct EmulationStats {
    pub frame_count: u64,
    pub total_cycles: usize,
    pub fps: f64,
}

impl EmulationRunner {
    pub fn new(driver: Driver, renderer: Renderer) -> Self {
        Self {
            driver,
            renderer,
            target_frame_duration: Duration::from_secs_f64(1.0 / 60.0),
        }
    }

    pub fn run<F>(&mut self, mut should_continue: F, debug_mode: bool) -> Result<(), String>
    where
        F: FnMut() -> bool,
    {
        let mut stats = EmulationStats {
            frame_count: 0,
            total_cycles: 0,
            fps: 0.0,
        };

        let mut last_fps_time = Instant::now();
        let mut last_debug_time = Instant::now();
        let mut fps_frame_count = 0u32;

        // Debug mode screenshot tracking
        let start_time = Instant::now();
        let mut screenshot_taken = false;

        println!("Emulator started. Press ESC to quit.");
        println!("Note: CPU implementation is incomplete, so execution may be limited.");

        // Initial render to show boot state
        self.renderer.render_frame()?;

        while should_continue() {
            let frame_start = Instant::now();

            // Run emulation for one frame (1/60 second worth of cycles)
            let cycles_executed = self.run_frame_cycles(&mut stats)?;

            // After running a frame's worth of cycles, we've completed one frame
            stats.frame_count += 1;
            fps_frame_count += 1;

            // Frame timing - wait for the remainder of 1/60 second
            let frame_duration = frame_start.elapsed();
            if frame_duration < self.target_frame_duration {
                std::thread::sleep(self.target_frame_duration - frame_duration);
            }

            // Update FPS every second
            let now = Instant::now();
            if now.duration_since(last_fps_time) >= Duration::from_secs(1) {
                stats.fps = fps_frame_count as f64;
                println!(
                    "FPS: {:.1}, Total frames: {}, Total cycles: {}",
                    stats.fps, stats.frame_count, stats.total_cycles
                );
                last_fps_time = now;
                fps_frame_count = 0;
            }

            // Debug output every few seconds
            if now.duration_since(last_debug_time) >= Duration::from_secs(5) {
                self.print_debug_info(cycles_executed);
                last_debug_time = now;
            }

            // Save screenshot after 5 seconds in debug mode
            if debug_mode
                && !screenshot_taken
                && now.duration_since(start_time) >= Duration::from_secs(5)
            {
                if let Err(e) = self.save_screenshot() {
                    println!("Warning: Failed to save screenshot: {}", e);
                } else {
                    println!("Debug: Screenshot saved");
                }
                screenshot_taken = true;
            }
        }

        Ok(())
    }

    fn run_frame_cycles(&mut self, stats: &mut EmulationStats) -> Result<usize, String> {
        let mut remaining_cycles = CYCLES_PER_FRAME;
        let mut total_cycles_this_frame = 0;

        // Keep running until we've executed all cycles for this frame
        while remaining_cycles > 0 {
            // Run for the remaining cycles
            let result = self.driver.run_for(remaining_cycles);

            // Track cycles executed
            let cycles_executed = result.cycles_executed;
            total_cycles_this_frame += cycles_executed;
            stats.total_cycles += cycles_executed;
            remaining_cycles = remaining_cycles.saturating_sub(cycles_executed);

            // Handle events
            match result.event {
                Event::VBlank => {
                    // Render the frame when PPU signals VBlank
                    self.renderer.render_frame()?;
                }
                Event::AudioBufferFull => {
                    // TODO: Handle audio when APU is implemented
                }
                Event::MaxCycles => {
                    // We've executed all requested cycles
                    // Due to a driver quirk, this might happen even with some cycles remaining
                    // if there are no futures scheduled exactly at those cycles
                    if remaining_cycles > 0 && cycles_executed == 0 {
                        // No progress made, skip remaining cycles
                        break;
                    }
                }
            }

            // Safety check: if no cycles were executed, something might be wrong
            if cycles_executed == 0 && remaining_cycles > 0 {
                // This can happen when there are no futures scheduled for the remaining cycles
                // For small numbers of remaining cycles, this is usually OK
                if remaining_cycles > 100 {
                    println!(
                        "Warning: No cycles executed with {} cycles remaining.",
                        remaining_cycles
                    );
                    println!("Debug: total_cycles = {}", stats.total_cycles);

                    // Try to render current state
                    self.renderer.render_frame()?;

                    return Err("Emulator appears to be stuck.".to_string());
                } else {
                    // Just skip the remaining cycles if it's a small amount
                    break;
                }
            }
        }

        Ok(total_cycles_this_frame)
    }

    fn print_debug_info(&self, frame_cycles: usize) {
        println!("Debug: Frame cycles: {}", frame_cycles);

        // Print PPU state
        let (ly, mode, lcdc) =
            with_state(|state| (state.ppu.ly, state.ppu.ppu_mode, state.ppu.lcdc));
        println!("Debug: PPU LY={}, Mode={}, LCDC=0x{:02X}", ly, mode, lcdc);
    }

    fn save_screenshot(&self) -> Result<(), String> {
        // Game Boy DMG color palette (greenish tint)
        const COLORS: [(u8, u8, u8); 4] = [
            (155, 188, 15), // Color 0 (lightest)
            (139, 172, 15), // Color 1
            (48, 98, 48),   // Color 2
            (15, 56, 15),   // Color 3 (darkest)
        ];

        // Get the framebuffer from PPU
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

        // Create an RGB image buffer
        let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(160, 144);

        // Convert Game Boy 2-bit colors to RGB
        for (y, row) in framebuffer.iter().enumerate() {
            for (x, &pixel) in row.iter().enumerate() {
                let color_index = pixel as usize;
                let (r, g, b) = COLORS[color_index.min(3)];
                img.put_pixel(x as u32, y as u32, Rgb([r, g, b]));
            }
        }

        // Generate filename with timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let filename = format!("gameboy_screenshot_{}.png", timestamp);

        // Save the image
        img.save(&filename)
            .map_err(|e| format!("Failed to save screenshot: {}", e))?;

        println!("Debug: Screenshot saved as {}", filename);
        Ok(())
    }
}
