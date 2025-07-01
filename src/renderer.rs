use crate::emulator::with_state;
use sdl2::event::Event as SdlEvent;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};
use sdl2::{EventPump, Sdl};
use std::collections::HashMap;

// Game Boy display constants
pub const GB_WIDTH: u32 = 160;
pub const GB_HEIGHT: u32 = 144;
pub const TILE_SIZE: u32 = 8;
pub const TILEMAP_WIDTH: u32 = 32;
pub const TILEMAP_HEIGHT: u32 = 32;
pub const TILEMAP_PIXEL_WIDTH: u32 = TILEMAP_WIDTH * TILE_SIZE; // 256
pub const TILEMAP_PIXEL_HEIGHT: u32 = TILEMAP_HEIGHT * TILE_SIZE; // 256

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

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
enum DebugWindowType {
    MainLcd,
    BgTilemap0,
    BgTilemap1,
    TileData,
    Palettes,
}

struct DebugWindow {
    canvas: WindowCanvas,
    texture: Texture<'static>,
}

pub struct Renderer {
    windows: HashMap<DebugWindowType, DebugWindow>,
    scale: u32,
}

impl Renderer {
    pub fn new(scale: u32, sdl_context: &Sdl) -> Result<Self, String> {
        let video_subsystem = sdl_context.video().map_err(|e| e.to_string())?;
        let mut windows = HashMap::new();

        // Get primary monitor bounds for window positioning
        let display_bounds = video_subsystem
            .display_bounds(0)
            .unwrap_or(Rect::new(0, 0, 1920, 1080));

        // Window arrangement calculations
        let main_width = GB_WIDTH * scale;
        let main_height = GB_HEIGHT * scale;
        let tilemap_scale = 1; // Tilemaps at 1x scale (256x256)
        let tilemap_width = TILEMAP_PIXEL_WIDTH * tilemap_scale;
        let tilemap_height = TILEMAP_PIXEL_HEIGHT * tilemap_scale;
        let tile_data_scale = 2; // Tile data at 2x scale
        let tile_data_width = 16 * TILE_SIZE * tile_data_scale; // 16 tiles wide, 2x scale = 256px
        let tile_data_height = 24 * TILE_SIZE * tile_data_scale; // 24 tiles tall, 2x scale = 384px
        let palette_width = 200;
        let palette_height = 100;
        let margin = 10;

        // Position calculations - arrange windows in a grid
        let center_x = display_bounds.width() as i32 / 2;
        let _center_y = display_bounds.height() as i32 / 2;

        // Main LCD window (top center)
        let main_x = center_x - (main_width as i32 / 2);
        let main_y = margin;

        // Tilemap windows (below main, side by side)
        let tilemap0_x = center_x - tilemap_width as i32 - margin / 2;
        let tilemap0_y = main_y + main_height as i32 + margin;

        let tilemap1_x = center_x + margin / 2;
        let tilemap1_y = tilemap0_y;

        // Tile data window (below tilemaps, left)
        let tile_data_x = tilemap0_x;
        let tile_data_y = tilemap0_y + tilemap_height as i32 + margin;

        // Palette window (below tilemaps, right)
        let palette_x = tilemap1_x;
        let palette_y = tile_data_y;

        // Create main LCD window
        let main_window = video_subsystem
            .window("GBEmu2 - LCD", main_width, main_height)
            .position(main_x, main_y)
            .build()
            .map_err(|e| e.to_string())?;
        let main_canvas = main_window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;
        let texture_creator = main_canvas.texture_creator();
        let main_texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, GB_WIDTH, GB_HEIGHT)
            .map_err(|e| e.to_string())?;
        windows.insert(
            DebugWindowType::MainLcd,
            DebugWindow {
                canvas: main_canvas,
                texture: unsafe { std::mem::transmute(main_texture) },
            },
        );

        // Create BG Tilemap 0 window
        let tilemap0_window = video_subsystem
            .window("GBEmu2 - BG Tilemap 0", tilemap_width, tilemap_height)
            .position(tilemap0_x, tilemap0_y)
            .build()
            .map_err(|e| e.to_string())?;
        let tilemap0_canvas = tilemap0_window
            .into_canvas()
            .build()
            .map_err(|e| e.to_string())?;
        let texture_creator = tilemap0_canvas.texture_creator();
        let tilemap0_texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                TILEMAP_PIXEL_WIDTH,
                TILEMAP_PIXEL_HEIGHT,
            )
            .map_err(|e| e.to_string())?;
        windows.insert(
            DebugWindowType::BgTilemap0,
            DebugWindow {
                canvas: tilemap0_canvas,
                texture: unsafe { std::mem::transmute(tilemap0_texture) },
            },
        );

        // Create BG Tilemap 1 window
        let tilemap1_window = video_subsystem
            .window("GBEmu2 - BG Tilemap 1", tilemap_width, tilemap_height)
            .position(tilemap1_x, tilemap1_y)
            .build()
            .map_err(|e| e.to_string())?;
        let tilemap1_canvas = tilemap1_window
            .into_canvas()
            .build()
            .map_err(|e| e.to_string())?;
        let texture_creator = tilemap1_canvas.texture_creator();
        let tilemap1_texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                TILEMAP_PIXEL_WIDTH,
                TILEMAP_PIXEL_HEIGHT,
            )
            .map_err(|e| e.to_string())?;
        windows.insert(
            DebugWindowType::BgTilemap1,
            DebugWindow {
                canvas: tilemap1_canvas,
                texture: unsafe { std::mem::transmute(tilemap1_texture) },
            },
        );

        // Create Tile Data window
        let tile_data_window = video_subsystem
            .window("GBEmu2 - Tile Data", tile_data_width, tile_data_height)
            .position(tile_data_x, tile_data_y)
            .build()
            .map_err(|e| e.to_string())?;
        let tile_data_canvas = tile_data_window
            .into_canvas()
            .build()
            .map_err(|e| e.to_string())?;
        let texture_creator = tile_data_canvas.texture_creator();
        let tile_data_texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, 16 * TILE_SIZE, 24 * TILE_SIZE)
            .map_err(|e| e.to_string())?;
        windows.insert(
            DebugWindowType::TileData,
            DebugWindow {
                canvas: tile_data_canvas,
                texture: unsafe { std::mem::transmute(tile_data_texture) },
            },
        );

        // Create Palettes window
        let palette_window = video_subsystem
            .window("GBEmu2 - Palettes", palette_width, palette_height)
            .position(palette_x, palette_y)
            .build()
            .map_err(|e| e.to_string())?;
        let palette_canvas = palette_window
            .into_canvas()
            .build()
            .map_err(|e| e.to_string())?;
        let texture_creator = palette_canvas.texture_creator();
        let palette_texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, palette_width, palette_height)
            .map_err(|e| e.to_string())?;
        windows.insert(
            DebugWindowType::Palettes,
            DebugWindow {
                canvas: palette_canvas,
                texture: unsafe { std::mem::transmute(palette_texture) },
            },
        );

        Ok(Renderer { windows, scale })
    }

    pub fn render_frame(&mut self) -> Result<(), String> {
        // Render main LCD
        self.render_main_lcd()?;

        // Render debug windows
        self.render_tilemap(DebugWindowType::BgTilemap0)?;
        self.render_tilemap(DebugWindowType::BgTilemap1)?;
        self.render_tile_data()?;
        self.render_palettes()?;

        Ok(())
    }

    fn render_main_lcd(&mut self) -> Result<(), String> {
        let window = self
            .windows
            .get_mut(&DebugWindowType::MainLcd)
            .ok_or("Main LCD window not found")?;

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
        window
            .texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..GB_HEIGHT as usize {
                    for x in 0..GB_WIDTH as usize {
                        let color_index = framebuffer[y][x] as usize;
                        let (r, g, b) = COLORS[color_index.min(3)];

                        let offset = y * pitch + x * 3;
                        buffer[offset] = r;
                        buffer[offset + 1] = g;
                        buffer[offset + 2] = b;
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        // Clear canvas and render texture
        window.canvas.clear();
        window
            .canvas
            .copy(
                &window.texture,
                None,
                Some(Rect::new(
                    0,
                    0,
                    GB_WIDTH * self.scale,
                    GB_HEIGHT * self.scale,
                )),
            )
            .map_err(|e| e.to_string())?;
        window.canvas.present();

        Ok(())
    }

    fn render_tilemap(&mut self, window_type: DebugWindowType) -> Result<(), String> {
        let window = self
            .windows
            .get_mut(&window_type)
            .ok_or("Tilemap window not found")?;

        // Get tilemap data and palette from PPU
        let tilemap_data = with_state(|state| {
            let tilemap = match window_type {
                DebugWindowType::BgTilemap0 => &state.ppu.bg_tile_map_0,
                DebugWindowType::BgTilemap1 => &state.ppu.bg_tile_map_1,
                _ => panic!("Invalid tilemap window type"),
            };

            let mut data = [[0u8; 256]; 256]; // Full tilemap in pixels
            let lcdc = state.ppu.lcdc;
            let bgp = state.ppu.bgp;

            // Determine tile data addressing mode
            let signed_addressing = (lcdc & 0x10) == 0;

            for tile_y in 0..32 {
                for tile_x in 0..32 {
                    let tile_index = tilemap[tile_y * 32 + tile_x];

                    // Calculate tile address based on addressing mode
                    let tile_num = if signed_addressing {
                        // Signed addressing: tiles -128 to 127, base at 0x9000
                        let signed_index = tile_index as i8 as i16;
                        (256 + signed_index) as usize
                    } else {
                        // Unsigned addressing: tiles 0 to 255, base at 0x8000
                        tile_index as usize
                    };

                    // Get tile data
                    for line in 0..8 {
                        let byte1 = state.ppu.vram[tile_num][line * 2];
                        let byte2 = state.ppu.vram[tile_num][line * 2 + 1];

                        for pixel in 0..8 {
                            let bit = 7 - pixel;
                            let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);

                            // Apply palette
                            let color = (bgp >> (color_num * 2)) & 0b11;

                            let x = tile_x * 8 + pixel;
                            let y = tile_y * 8 + line;
                            data[y][x] = color;
                        }
                    }
                }
            }

            data
        });

        // Convert to RGB and render
        window
            .texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..256 {
                    for x in 0..256 {
                        let color_index = tilemap_data[y][x] as usize;
                        let (r, g, b) = COLORS[color_index.min(3)];

                        let offset = y * pitch + x * 3;
                        buffer[offset] = r;
                        buffer[offset + 1] = g;
                        buffer[offset + 2] = b;
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        window.canvas.clear();
        window
            .canvas
            .copy(
                &window.texture,
                None,
                Some(Rect::new(
                    0,
                    0,
                    TILEMAP_PIXEL_WIDTH * 1,
                    TILEMAP_PIXEL_HEIGHT * 1,
                )),
            )
            .map_err(|e| e.to_string())?;
        window.canvas.present();

        Ok(())
    }

    fn render_tile_data(&mut self) -> Result<(), String> {
        let window = self
            .windows
            .get_mut(&DebugWindowType::TileData)
            .ok_or("Tile data window not found")?;

        // Get tile data from PPU
        let tile_data = with_state(|state| {
            let mut data = [[0u8; 16 * 8]; 24 * 8]; // 16x24 tiles, 8x8 pixels each
            let bgp = state.ppu.bgp;

            // Render all 384 tiles
            for tile_idx in 0..384 {
                let tile_x = tile_idx % 16;
                let tile_y = tile_idx / 16;

                if tile_y >= 24 {
                    break; // Only display first 384 tiles
                }

                // Get tile data
                for line in 0..8 {
                    let byte1 = state.ppu.vram[tile_idx][line * 2];
                    let byte2 = state.ppu.vram[tile_idx][line * 2 + 1];

                    for pixel in 0..8 {
                        let bit = 7 - pixel;
                        let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);

                        // Apply palette
                        let color = (bgp >> (color_num * 2)) & 0b11;

                        let x = tile_x * 8 + pixel;
                        let y = tile_y * 8 + line;
                        data[y][x] = color;
                    }
                }
            }

            data
        });

        // Convert to RGB and render
        window
            .texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..(24 * 8) {
                    for x in 0..(16 * 8) {
                        let color_index = tile_data[y][x] as usize;
                        let (r, g, b) = COLORS[color_index.min(3)];

                        let offset = y * pitch + x * 3;
                        buffer[offset] = r;
                        buffer[offset + 1] = g;
                        buffer[offset + 2] = b;
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        window.canvas.clear();
        window
            .canvas
            .copy(
                &window.texture,
                None,
                Some(Rect::new(0, 0, 16 * TILE_SIZE * 2, 24 * TILE_SIZE * 2)),
            )
            .map_err(|e| e.to_string())?;
        window.canvas.present();

        Ok(())
    }

    fn render_palettes(&mut self) -> Result<(), String> {
        let window = self
            .windows
            .get_mut(&DebugWindowType::Palettes)
            .ok_or("Palettes window not found")?;

        // Get palette data from PPU
        let (bgp, obp0, obp1) = with_state(|state| (state.ppu.bgp, state.ppu.obp0, state.ppu.obp1));

        // Render palettes
        window
            .texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                // Clear to dark gray
                for y in 0..100 {
                    for x in 0..200 {
                        let offset = y * pitch + x * 3;
                        buffer[offset] = 40;
                        buffer[offset + 1] = 40;
                        buffer[offset + 2] = 40;
                    }
                }

                // Draw palette squares (40x40 each)
                let draw_palette = |buffer: &mut [u8],
                                    pitch: usize,
                                    palette: u8,
                                    y_offset: usize,
                                    _label: &str| {
                    for i in 0..4 {
                        let color_num = (palette >> (i * 2)) & 0b11;
                        let (r, g, b) = COLORS[color_num as usize];

                        let x_start = 10 + i * 45;
                        let y_start = y_offset;

                        // Draw color square
                        for y in y_start..(y_start + 25) {
                            for x in x_start..(x_start + 40) {
                                if x < 200 && y < 100 {
                                    let offset = y * pitch + x * 3;
                                    buffer[offset] = r;
                                    buffer[offset + 1] = g;
                                    buffer[offset + 2] = b;
                                }
                            }
                        }
                    }
                };

                // Draw BGP palette
                draw_palette(buffer, pitch, bgp, 10, "BGP");

                // Draw OBP0 palette
                draw_palette(buffer, pitch, obp0, 40, "OBP0");

                // Draw OBP1 palette
                draw_palette(buffer, pitch, obp1, 70, "OBP1");
            })
            .map_err(|e| e.to_string())?;

        window.canvas.clear();
        window
            .canvas
            .copy(&window.texture, None, None)
            .map_err(|e| e.to_string())?;
        window.canvas.present();

        Ok(())
    }
}
