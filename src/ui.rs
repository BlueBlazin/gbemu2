use crate::driver::Event;
use crate::emulator::{create_driver, run_emulator_step, with_state};
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event as WinitEvent, WindowEvent};
use winit::event::{KeyboardInput, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;
const SCALE_FACTOR: u32 = 4;

pub fn run_ui() {
    let mut driver = create_driver();

    let event_loop = EventLoop::new();
    let window_size = LogicalSize::new(SCREEN_WIDTH * SCALE_FACTOR, SCREEN_HEIGHT * SCALE_FACTOR);

    let window = WindowBuilder::new()
        .with_title("Game Boy Emulator")
        .with_inner_size(window_size)
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture).unwrap();

    let mut paused = false;

    event_loop.run(move |event, _, control_flow| {
        match event {
            WinitEvent::RedrawRequested(_) => {
                if let Err(err) = pixels.render() {
                    eprintln!("pixels.render() failed: {err}");
                }
            }
            WinitEvent::MainEventsCleared => {
                if !paused {
                    let result = run_emulator_step(&mut driver);

                    // Update display only when VBlank occurs
                    if let Event::VBlank = result.event {
                        update_display(&mut pixels);
                    }
                }

                window.request_redraw();
            }
            WinitEvent::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => match keycode {
                    VirtualKeyCode::Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    VirtualKeyCode::Space => {
                        paused = !paused;
                        println!("Emulator {}", if paused { "paused" } else { "resumed" });
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }

        *control_flow = ControlFlow::Poll;
    });
}

fn update_display(pixels: &mut Pixels) {
    let frame = pixels.get_frame_mut();

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
