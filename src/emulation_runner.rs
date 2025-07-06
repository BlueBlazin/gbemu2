use crate::driver::{Driver, Event};
use crate::emulator::{with_state, with_state_mut};
use crate::renderer::Renderer;
use image::{ImageBuffer, Rgb};
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const GB_CLOCK_SPEED: f64 = 4_194_304.0; // Hz
pub const CYCLES_PER_FRAME: usize = (GB_CLOCK_SPEED / 60.0) as usize; // ~69905 cycles

pub struct EmulationRunner {
    driver: Driver,
    renderer: Renderer,
    target_frame_duration: Duration,
    cycle_correction: isize, // Track cycle overshoot/undershoot from previous frames
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
            cycle_correction: 0,
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

        // Timing measurement for optimization
        let mut frame_time_sum = Duration::ZERO;
        let mut frame_time_count = 0u32;
        let mut last_timing_print = Instant::now();

        // Debug mode screenshot tracking
        let start_time = Instant::now();
        let mut screenshot_taken = false;

        println!("Emulator started. Press ESC to quit.");

        // Initial render to show boot state
        self.renderer.render_frame()?;
        // Clear dirty flags after initial render
        with_state_mut(|state| state.clear_all_dirty_flags());

        while should_continue() {
            let frame_start = Instant::now();

            // Run emulation for one frame (1/60 second worth of cycles)
            let frame_cycles_start = Instant::now();
            let cycles_executed = self.run_frame_cycles(&mut stats)?;
            let frame_cycles_duration = frame_cycles_start.elapsed();

            // Track timing for optimization measurement
            frame_time_sum += frame_cycles_duration;
            frame_time_count += 1;

            // After running a frame's worth of cycles, we've completed one frame
            stats.frame_count += 1;
            fps_frame_count += 1;

            // Frame timing - wait for the remainder of 1/60 second
            let frame_duration = frame_start.elapsed();
            if frame_duration < self.target_frame_duration {
                std::thread::sleep(self.target_frame_duration - frame_duration);
            }

            // Print timing stats every 5 seconds (not debug mode specific)
            let now = Instant::now();
            if now.duration_since(last_timing_print) >= Duration::from_secs(5) {
                if frame_time_count > 0 {
                    let avg_frame_time = frame_time_sum / frame_time_count;
                    println!(
                        "Avg frame emulation time: {:.3}ms ({} frames sampled)",
                        avg_frame_time.as_secs_f64() * 1000.0,
                        frame_time_count
                    );
                    // Reset counters
                    frame_time_sum = Duration::ZERO;
                    frame_time_count = 0;
                }
                last_timing_print = now;
            }

            if debug_mode {
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
                if !screenshot_taken && now.duration_since(start_time) >= Duration::from_secs(5) {
                    if let Err(e) = self.save_screenshot() {
                        println!("Warning: Failed to save screenshot: {}", e);
                    } else {
                        println!("Debug: Screenshot saved");
                    }
                    screenshot_taken = true;
                }
            }
        }

        // Write debug log if in debug mode
        if debug_mode {
            self.write_debug_log()?;
        }

        Ok(())
    }

    fn run_frame_cycles(&mut self, stats: &mut EmulationStats) -> Result<usize, String> {
        // Calculate target cycles for this frame, accounting for previous overshoot/undershoot
        let target_cycles = (CYCLES_PER_FRAME as isize - self.cycle_correction) as usize;
        let mut remaining_cycles = target_cycles;
        let mut total_cycles_this_frame = 0;

        // Keep running until MaxCycles event. MaxCycles event doesn't guarantee target_cycles
        // number of cycles were run but rather that running any more cycles would've crossed that boundary.
        loop {
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
                    // Clear dirty flags after successful rendering
                    with_state_mut(|state| state.clear_all_dirty_flags());
                }
                Event::AudioBufferFull => {
                    // Drain the audio buffer and send to audio system
                    let audio_buffer = with_state_mut(|state| state.apu.drain_buffer());
                    if !audio_buffer.is_empty() {
                        self.renderer.queue_audio_samples(&audio_buffer);
                    }
                }
                Event::MaxCycles => {
                    // Calculate how many cycles we overshot or undershot by
                    let overshoot = total_cycles_this_frame as isize - CYCLES_PER_FRAME as isize;
                    self.cycle_correction = overshoot;

                    // We've executed all requested cycles
                    return Ok(total_cycles_this_frame);
                }
            }
        }
    }

    fn print_debug_info(&self, frame_cycles: usize) {
        println!(
            "Debug: Frame cycles: {} (correction: {})",
            frame_cycles, self.cycle_correction
        );

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

    fn write_debug_log(&self) -> Result<(), String> {
        // Get the opcode log from state
        let opcode_log = with_state(|state| state.opcode_log.clone());

        // Create debug.txt file
        let mut file =
            File::create("debug.txt").map_err(|e| format!("Failed to create debug.txt: {}", e))?;

        // Write header
        writeln!(&mut file, "Game Boy Emulator Debug Log")
            .map_err(|e| format!("Failed to write to debug.txt: {}", e))?;
        writeln!(&mut file, "Total opcodes executed: {}", opcode_log.len())
            .map_err(|e| format!("Failed to write to debug.txt: {}", e))?;
        writeln!(&mut file, "\nPC    | Opcode | Instruction")
            .map_err(|e| format!("Failed to write to debug.txt: {}", e))?;
        writeln!(&mut file, "------|--------|-------------")
            .map_err(|e| format!("Failed to write to debug.txt: {}", e))?;

        // Write each opcode
        for (pc, opcode) in opcode_log.iter() {
            writeln!(
                &mut file,
                "0x{:04X} | 0x{:02X}   | {}",
                pc,
                opcode,
                get_instruction_name(*opcode)
            )
            .map_err(|e| format!("Failed to write to debug.txt: {}", e))?;
        }

        println!("Debug: Opcode log saved to debug.txt");
        Ok(())
    }
}

// Helper function to get instruction name (basic version)
fn get_instruction_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "NOP",
        0x01 => "LD BC,nn",
        0x02 => "LD (BC),A",
        0x03 => "INC BC",
        0x04 => "INC B",
        0x05 => "DEC B",
        0x06 => "LD B,n",
        0x07 => "RLCA",
        0x08 => "LD (nn),SP",
        0x09 => "ADD HL,BC",
        0x0A => "LD A,(BC)",
        0x0B => "DEC BC",
        0x0C => "INC C",
        0x0D => "DEC C",
        0x0E => "LD C,n",
        0x0F => "RRCA",

        0x10 => "STOP",
        0x11 => "LD DE,nn",
        0x12 => "LD (DE),A",
        0x13 => "INC DE",
        0x14 => "INC D",
        0x15 => "DEC D",
        0x16 => "LD D,n",
        0x17 => "RLA",
        0x18 => "JR n",
        0x19 => "ADD HL,DE",
        0x1A => "LD A,(DE)",
        0x1B => "DEC DE",
        0x1C => "INC E",
        0x1D => "DEC E",
        0x1E => "LD E,n",
        0x1F => "RRA",

        0x20 => "JR NZ,n",
        0x21 => "LD HL,nn",
        0x22 => "LD (HL+),A",
        0x23 => "INC HL",
        0x24 => "INC H",
        0x25 => "DEC H",
        0x26 => "LD H,n",
        0x27 => "DAA",
        0x28 => "JR Z,n",
        0x29 => "ADD HL,HL",
        0x2A => "LD A,(HL+)",
        0x2B => "DEC HL",
        0x2C => "INC L",
        0x2D => "DEC L",
        0x2E => "LD L,n",
        0x2F => "CPL",

        0x30 => "JR NC,n",
        0x31 => "LD SP,nn",
        0x32 => "LD (HL-),A",
        0x33 => "INC SP",
        0x34 => "INC (HL)",
        0x35 => "DEC (HL)",
        0x36 => "LD (HL),n",
        0x37 => "SCF",
        0x38 => "JR C,n",
        0x39 => "ADD HL,SP",
        0x3A => "LD A,(HL-)",
        0x3B => "DEC SP",
        0x3C => "INC A",
        0x3D => "DEC A",
        0x3E => "LD A,n",
        0x3F => "CCF",

        0x40..=0x7F => {
            if opcode == 0x76 {
                "HALT"
            } else {
                "LD r,r"
            }
        }

        0x80..=0x87 => "ADD A,r",
        0x88..=0x8F => "ADC A,r",
        0x90..=0x97 => "SUB r",
        0x98..=0x9F => "SBC A,r",
        0xA0..=0xA7 => "AND r",
        0xA8..=0xAF => "XOR r",
        0xB0..=0xB7 => "OR r",
        0xB8..=0xBF => "CP r",

        0xC0 => "RET NZ",
        0xC1 => "POP BC",
        0xC2 => "JP NZ,nn",
        0xC3 => "JP nn",
        0xC4 => "CALL NZ,nn",
        0xC5 => "PUSH BC",
        0xC6 => "ADD A,n",
        0xC7 => "RST 00H",
        0xC8 => "RET Z",
        0xC9 => "RET",
        0xCA => "JP Z,nn",
        0xCB => "CB prefix",
        0xCC => "CALL Z,nn",
        0xCD => "CALL nn",
        0xCE => "ADC A,n",
        0xCF => "RST 08H",

        0xD0 => "RET NC",
        0xD1 => "POP DE",
        0xD2 => "JP NC,nn",
        0xD4 => "CALL NC,nn",
        0xD5 => "PUSH DE",
        0xD6 => "SUB n",
        0xD7 => "RST 10H",
        0xD8 => "RET C",
        0xD9 => "RETI",
        0xDA => "JP C,nn",
        0xDC => "CALL C,nn",
        0xDE => "SBC A,n",
        0xDF => "RST 18H",

        0xE0 => "LDH (n),A",
        0xE1 => "POP HL",
        0xE2 => "LD (C),A",
        0xE5 => "PUSH HL",
        0xE6 => "AND n",
        0xE7 => "RST 20H",
        0xE8 => "ADD SP,n",
        0xE9 => "JP (HL)",
        0xEA => "LD (nn),A",
        0xEE => "XOR n",
        0xEF => "RST 28H",

        0xF0 => "LDH A,(n)",
        0xF1 => "POP AF",
        0xF2 => "LD A,(C)",
        0xF3 => "DI",
        0xF5 => "PUSH AF",
        0xF6 => "OR n",
        0xF7 => "RST 30H",
        0xF8 => "LD HL,SP+n",
        0xF9 => "LD SP,HL",
        0xFA => "LD A,(nn)",
        0xFB => "EI",
        0xFE => "CP n",
        0xFF => "RST 38H",

        _ => "Unknown",
    }
}
