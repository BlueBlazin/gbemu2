use crate::emulator::with_state;
use sdl2::event::Event as SdlEvent;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};
use sdl2::{EventPump, Sdl};

// Game Boy display constants
pub const GB_WIDTH: u32 = 160;
pub const GB_HEIGHT: u32 = 144;

// Game Boy DMG color palette (greenish tint)
const COLORS: [(u8, u8, u8); 4] = [
    (155, 188, 15), // Color 0 (lightest)
    (139, 172, 15), // Color 1
    (48, 98, 48),   // Color 2
    (15, 56, 15),   // Color 3 (darkest)
];

pub struct SdlSystem {
    _context: Sdl,
    event_pump: EventPump,
}

impl SdlSystem {
    pub fn new() -> Result<(Self, Sdl), String> {
        let sdl_context = sdl2::init().map_err(|e| e.to_string())?;
        let event_pump = sdl_context.event_pump().map_err(|e| e.to_string())?;

        let system = SdlSystem {
            _context: sdl_context.clone(),
            event_pump,
        };

        Ok((system, sdl_context))
    }

    pub fn poll_events(&mut self) -> bool {
        for event in self.event_pump.poll_iter() {
            match event {
                SdlEvent::Quit { .. } => return false,
                SdlEvent::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return false,
                _ => {}
            }
        }
        true
    }
}

pub struct Renderer {
    canvas: WindowCanvas,
    texture: Texture<'static>,
    scale: u32,
}

impl Renderer {
    pub fn new(scale: u32, sdl_context: &Sdl) -> Result<Self, String> {
        let video_subsystem = sdl_context.video().map_err(|e| e.to_string())?;

        let window_width = GB_WIDTH * scale;
        let window_height = GB_HEIGHT * scale;

        let window = video_subsystem
            .window("GBEmu2", window_width, window_height)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        let texture_creator = canvas.texture_creator();
        let texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, GB_WIDTH, GB_HEIGHT)
            .map_err(|e| e.to_string())?;

        // SAFETY: We need to transmute the lifetime to 'static because SDL2's types
        // don't properly express the lifetime relationship. The texture is valid as
        // long as the canvas exists, and we keep both in the same struct.
        let texture = unsafe { std::mem::transmute(texture) };

        Ok(Renderer {
            canvas,
            texture,
            scale,
        })
    }

    pub fn render_frame(&mut self) -> Result<(), String> {
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
        self.texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..GB_HEIGHT as usize {
                    for x in 0..GB_WIDTH as usize {
                        let color_index = framebuffer[y][x] as usize;
                        let (r, g, b) = COLORS[color_index.min(3)]; // Ensure valid index

                        let offset = y * pitch + x * 3;
                        buffer[offset] = r;
                        buffer[offset + 1] = g;
                        buffer[offset + 2] = b;
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        // Clear canvas and render texture
        self.canvas.clear();
        self.canvas
            .copy(
                &self.texture,
                None,
                Some(Rect::new(
                    0,
                    0,
                    GB_WIDTH * self.scale,
                    GB_HEIGHT * self.scale,
                )),
            )
            .map_err(|e| e.to_string())?;
        self.canvas.present();

        Ok(())
    }
}
