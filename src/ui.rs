use crate::driver::Event;
use crate::emulator::{create_driver, with_state};
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;
const SCALE_FACTOR: u32 = 3;

// Game Boy runs at 4.194304 MHz, at 60 FPS = 69905 cycles per frame
const CYCLES_PER_FRAME: usize = 69905;
const FRAME_DURATION: Duration = Duration::from_nanos(16_666_667); // ~60 FPS (1/60 second)

struct PixelsWrapper {
    pixels: Pixels<'static>,
}

struct EmulatorApp {
    window: Option<Arc<Window>>,
    pixels_wrapper: Option<Box<PixelsWrapper>>,
    driver: crate::driver::Driver,
    last_frame_time: Instant,
}

impl EmulatorApp {
    fn new() -> Self {
        Self {
            window: None,
            pixels_wrapper: None,
            driver: create_driver(),
            last_frame_time: Instant::now(),
        }
    }

    fn run_frame(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);

        if elapsed >= FRAME_DURATION {
            // Start a new frame
            self.last_frame_time = now;
            let mut cycles_remaining = CYCLES_PER_FRAME;

            // Run the emulator for one frame worth of cycles
            while cycles_remaining > 0 {
                let result = self.driver.run_for(cycles_remaining);

                // Deduct the cycles we just executed
                cycles_remaining = cycles_remaining.saturating_sub(result.cycles_executed);

                // Handle the event that caused run_for to return
                match result.event {
                    Event::VBlank => {
                        // Update the display
                        if let Some(wrapper) = &mut self.pixels_wrapper {
                            update_display(&mut wrapper.pixels);
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    Event::AudioBufferFull => {
                        // TODO: Handle audio when APU is implemented
                    }
                    Event::MaxCycles => {
                        // We've executed all requested cycles
                        // This is the normal case when no events occurred
                        return;
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for EmulatorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Window resumed, creating window...");
        // Create window when application is resumed
        let window_size =
            LogicalSize::new(SCREEN_WIDTH * SCALE_FACTOR, SCREEN_HEIGHT * SCALE_FACTOR);

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Game Boy Emulator")
                        .with_inner_size(window_size)
                        .with_resizable(false),
                )
                .unwrap(),
        );

        println!("Window created successfully");

        // Create pixels renderer
        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.clone());
        let pixels = Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture).unwrap();

        println!("Pixels renderer created");

        self.window = Some(window);
        self.pixels_wrapper = Some(Box::new(PixelsWrapper { pixels }));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Render the current frame
                if let Some(wrapper) = &mut self.pixels_wrapper {
                    if let Err(err) = wrapper.pixels.render() {
                        eprintln!("pixels.render() failed: {err}");
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Run emulation for one frame if it's time
        self.run_frame();

        // Set next frame timing
        let next_frame_time = self.last_frame_time + FRAME_DURATION;
        let now = Instant::now();

        if now < next_frame_time {
            // Wait until next frame
            event_loop.set_control_flow(ControlFlow::wait_duration(next_frame_time - now));
        } else {
            // We're behind schedule, run immediately
            event_loop.set_control_flow(ControlFlow::Poll);
        }
    }
}

pub fn run_ui() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = EmulatorApp::new();
    let _ = event_loop.run_app(&mut app);
}

fn update_display(pixels: &mut Pixels) {
    let frame = pixels.frame_mut();

    // Get framebuffer from PPU state
    with_state(|state| {
        let framebuffer = &state.ppu.framebuffer;

        // Convert Game Boy 2-bit colors to RGBA
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let y = i / SCREEN_WIDTH as usize;
            let x = i % SCREEN_WIDTH as usize;

            if y < SCREEN_HEIGHT as usize && x < SCREEN_WIDTH as usize {
                let gb_color = framebuffer[y][x];
                let rgba = gameboy_color_to_rgba(gb_color);
                pixel.copy_from_slice(&rgba);
            }
        }
    });
}

fn gameboy_color_to_rgba(gb_color: u8) -> [u8; 4] {
    // Convert Game Boy 2-bit color (0-3) to RGBA
    // Using classic Game Boy green tints
    match gb_color & 0x03 {
        0 => [155, 188, 15, 255], // Lightest green (white)
        1 => [139, 172, 15, 255], // Light green
        2 => [48, 98, 48, 255],   // Dark green
        3 => [15, 56, 15, 255],   // Darkest green (black)
        _ => [0, 0, 0, 255],      // Should never happen
    }
}
