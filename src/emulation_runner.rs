use crate::driver::{Driver, Event};
use crate::emulator::with_state;
use crate::renderer::Renderer;
use std::time::{Duration, Instant};

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

    pub fn run<F>(&mut self, mut should_continue: F) -> Result<(), String>
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

        println!("Emulator started. Press ESC to quit.");
        println!("Note: CPU implementation is incomplete, so execution may be limited.");

        // Initial render to show boot state
        self.renderer.render_frame()?;

        while should_continue() {
            let frame_start = Instant::now();

            // Run one frame
            let frame_cycles = self.run_frame(&mut stats)?;
            fps_frame_count += 1;

            // Frame timing
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
                self.print_debug_info(frame_cycles);
                last_debug_time = now;
            }
        }

        Ok(())
    }

    fn run_frame(&mut self, stats: &mut EmulationStats) -> Result<usize, String> {
        let mut vblank_occurred = false;
        let mut frame_cycles = 0usize;

        // Keep running until we get a VBlank
        while !vblank_occurred {
            // Run for a batch of cycles
            let result = self.driver.run_for(10000); // Run for 10k cycles at a time
            frame_cycles += result.cycles_executed;
            stats.total_cycles += result.cycles_executed;

            match result.event {
                Event::VBlank => {
                    // Render the frame
                    self.renderer.render_frame()?;
                    vblank_occurred = true;
                    stats.frame_count += 1;
                }
                Event::AudioBufferFull => {
                    // TODO: Handle audio when APU is implemented
                }
                Event::MaxCycles => {
                    // Continue running
                }
            }
        }

        Ok(frame_cycles)
    }

    fn print_debug_info(&self, frame_cycles: usize) {
        println!("Debug: Frame cycles: {}", frame_cycles);

        // Print PPU state
        let (ly, mode, lcdc) =
            with_state(|state| (state.ppu.ly, state.ppu.ppu_mode, state.ppu.lcdc));
        println!("Debug: PPU LY={}, Mode={}, LCDC=0x{:02X}", ly, mode, lcdc);
    }
}
