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
    (117, 152, 51), // Color 0 (lightest)
    (88, 143, 81),  // Color 1
    (60, 118, 96),  // Color 2
    (46, 97, 90),   // Color 3 (darkest)
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
    SpriteTiles,
}

struct DebugWindow {
    canvas: WindowCanvas,
    texture: Texture<'static>,
}

pub struct Renderer {
    windows: HashMap<DebugWindowType, DebugWindow>,
    scale: u32,
    grid_lines: Vec<(sdl2::rect::Point, sdl2::rect::Point)>,
    border_horizontal: u32,
    border_vertical: u32,
}

impl Renderer {
    pub fn new(scale: u32, sdl_context: &Sdl, rom_filename: &str) -> Result<Self, String> {
        let video_subsystem = sdl_context.video().map_err(|e| e.to_string())?;
        let mut windows = HashMap::new();

        // Get primary monitor bounds for window positioning
        let display_bounds = video_subsystem
            .display_bounds(0)
            .unwrap_or(Rect::new(0, 0, 1920, 1080));

        // Window arrangement calculations
        let lcd_content_width = GB_WIDTH * scale;
        let lcd_content_height = GB_HEIGHT * scale;
        let border_vertical = 20; // Top and bottom border thickness
        let border_horizontal = 40; // Left and right border thickness (twice as thick)
        let main_width = lcd_content_width + (border_horizontal * 2);
        let main_height = lcd_content_height + (border_vertical * 2);
        let tilemap_scale = 1; // Tilemaps at 1x scale (256x256)
        let tilemap_width = TILEMAP_PIXEL_WIDTH * tilemap_scale;
        let tilemap_height = TILEMAP_PIXEL_HEIGHT * tilemap_scale;
        let tile_data_scale = 2; // Tile data at 2x scale
        let tile_data_width = 16 * TILE_SIZE * tile_data_scale; // 16 tiles wide, 2x scale = 256px
        let tile_data_height = 24 * TILE_SIZE * tile_data_scale; // 24 tiles tall, 2x scale = 384px
        let palette_width = 200;
        let palette_height = 100;
        let margin = 10;

        // Position calculations - arrange windows as requested:
        // Main LCD in center, BG tilemaps stacked on right, Tile data on left, Palettes below tile data
        let center_x = display_bounds.width() as i32 / 2;
        let center_y = display_bounds.height() as i32 / 2;

        // Main LCD window (center)
        let main_x = center_x - (main_width as i32 / 2);
        let main_y = center_y - (main_height as i32 / 2);

        // Tile data window (left of main LCD, moved up to account for title bar)
        let tile_data_x = main_x - tile_data_width as i32 - margin;
        let tile_data_y = main_y - 30;

        // BG Tilemap windows (right of main LCD, stacked vertically)
        let tilemap0_x = main_x + main_width as i32 + margin;
        let tilemap0_y = main_y - 30;

        let tilemap1_x = tilemap0_x;
        let tilemap1_y = tilemap0_y + tilemap_height as i32 + margin + 30;

        // Sprite Tiles window (right of BG tilemaps) - 10x4 grid at 3x scale
        let sprite_tiles_scale = 5; // 5x scale for good visibility
        let sprite_tiles_width = 4 * TILE_SIZE * sprite_tiles_scale; // 4 sprites wide at 3x scale
        let sprite_tiles_height = 10 * TILE_SIZE * sprite_tiles_scale; // 10 sprites tall at 3x scale
        let sprite_tiles_x = tilemap0_x + tilemap_width as i32 + margin;
        let sprite_tiles_y = tilemap0_y;

        // Palette window (below tile data)
        let palette_x = tile_data_x;
        let palette_y = tile_data_y + tile_data_height as i32 + margin + 30;

        // Create main LCD window with ROM filename as title
        let main_window = video_subsystem
            .window(rom_filename, main_width, main_height)
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
            .window("BG Tilemap 0", tilemap_width, tilemap_height)
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
            .window("BG Tilemap 1", tilemap_width, tilemap_height)
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
            .window("Tile Data", tile_data_width, tile_data_height)
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
            .window("Palettes", palette_width, palette_height)
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

        // Create Sprite Tiles window
        let sprite_tiles_window = video_subsystem
            .window("Sprite Tiles", sprite_tiles_width, sprite_tiles_height)
            .position(sprite_tiles_x, sprite_tiles_y)
            .build()
            .map_err(|e| e.to_string())?;
        let sprite_tiles_canvas = sprite_tiles_window
            .into_canvas()
            .build()
            .map_err(|e| e.to_string())?;
        let texture_creator = sprite_tiles_canvas.texture_creator();
        let sprite_tiles_texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, 4 * TILE_SIZE, 10 * TILE_SIZE)
            .map_err(|e| e.to_string())?;
        windows.insert(
            DebugWindowType::SpriteTiles,
            DebugWindow {
                canvas: sprite_tiles_canvas,
                texture: unsafe { std::mem::transmute(sprite_tiles_texture) },
            },
        );

        // Pre-calculate grid line coordinates (offset by border)
        let mut grid_lines = Vec::new();
        let border_h = border_horizontal as i32;
        let border_v = border_vertical as i32;

        // Vertical lines (159 lines between 160 columns)
        for x in 1..GB_WIDTH {
            let x_pos = border_h + (x * scale) as i32;
            grid_lines.push((
                sdl2::rect::Point::new(x_pos, border_v),
                sdl2::rect::Point::new(x_pos, border_v + (GB_HEIGHT * scale) as i32),
            ));
        }

        // Horizontal lines (143 lines between 144 rows)
        for y in 1..GB_HEIGHT {
            let y_pos = border_v + (y * scale) as i32;
            grid_lines.push((
                sdl2::rect::Point::new(border_h, y_pos),
                sdl2::rect::Point::new(border_h + (GB_WIDTH * scale) as i32, y_pos),
            ));
        }

        Ok(Renderer {
            windows,
            scale,
            grid_lines,
            border_horizontal,
            border_vertical,
        })
    }

    pub fn render_frame(&mut self) -> Result<(), String> {
        // Render main LCD
        self.render_main_lcd()?;

        // Render debug windows
        self.render_tilemap(DebugWindowType::BgTilemap0)?;
        self.render_tilemap(DebugWindowType::BgTilemap1)?;
        self.render_tile_data()?;
        self.render_palettes()?;
        self.render_sprite_tiles()?;

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

        // Clear canvas with light grayish-purple color (flat border)
        window
            .canvas
            // .set_draw_color(sdl2::pixels::Color::RGB(170, 160, 172));
            // .set_draw_color(sdl2::pixels::Color::RGB(61, 64, 70));
            .set_draw_color(sdl2::pixels::Color::RGB(32, 33, 36));
        window.canvas.clear();

        // Create sunken effect only around the LCD screen
        let bh = self.border_horizontal as i32;
        let bv = self.border_vertical as i32;
        let content_width = (GB_WIDTH * self.scale) as i32;
        let content_height = (GB_HEIGHT * self.scale) as i32;

        // Draw inner shadow (top and left edges around LCD for sunken effect)
        window
            .canvas
            .set_draw_color(sdl2::pixels::Color::RGB(15, 15, 15));
        // Top edge of LCD area
        for i in 0..3 {
            window
                .canvas
                .draw_line(
                    sdl2::rect::Point::new(bh - 3, bv - 3 + i),
                    sdl2::rect::Point::new(bh + content_width + 2, bv - 3 + i),
                )
                .map_err(|e| e.to_string())?;
        }
        // Left edge of LCD area
        for i in 0..3 {
            window
                .canvas
                .draw_line(
                    sdl2::rect::Point::new(bh - 3 + i, bv - 3),
                    sdl2::rect::Point::new(bh - 3 + i, bv + content_height + 2),
                )
                .map_err(|e| e.to_string())?;
        }

        // Draw inner highlight (bottom and right edges around LCD for sunken effect)
        window
            .canvas
            .set_draw_color(sdl2::pixels::Color::RGB(45, 45, 45));
        // Bottom edge of LCD area
        for i in 0..3 {
            let y = bv + content_height + 2 - i;
            window
                .canvas
                .draw_line(
                    sdl2::rect::Point::new(bh - 3, y),
                    sdl2::rect::Point::new(bh + content_width + 2, y),
                )
                .map_err(|e| e.to_string())?;
        }
        // Right edge of LCD area
        for i in 0..3 {
            let x = bh + content_width + 2 - i;
            window
                .canvas
                .draw_line(
                    sdl2::rect::Point::new(x, bv - 3),
                    sdl2::rect::Point::new(x, bv + content_height + 2),
                )
                .map_err(|e| e.to_string())?;
        }

        // Render LCD content texture with border offset
        window
            .canvas
            .copy(
                &window.texture,
                None,
                Some(Rect::new(
                    self.border_horizontal as i32,
                    self.border_vertical as i32,
                    GB_WIDTH * self.scale,
                    GB_HEIGHT * self.scale,
                )),
            )
            .map_err(|e| e.to_string())?;

        // Draw pixel grid overlay using pre-calculated lines
        use sdl2::render::BlendMode;

        // Enable alpha blending for the canvas
        window.canvas.set_blend_mode(BlendMode::Blend);

        // Use semi-transparent color (alpha = 50 out of 255 for subtle grid)
        window
            .canvas
            .set_draw_color(sdl2::pixels::Color::RGBA(148, 138, 5, 50));

        // Draw all pre-calculated grid lines
        for (start, end) in &self.grid_lines {
            window
                .canvas
                .draw_line(*start, *end)
                .map_err(|e| e.to_string())?;
        }

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

    fn render_sprite_tiles(&mut self) -> Result<(), String> {
        let window = self
            .windows
            .get_mut(&DebugWindowType::SpriteTiles)
            .ok_or("Sprite tiles window not found")?;

        // Get sprite tile data from PPU
        let sprite_data = with_state(|state| {
            let mut data = [[0u8; 4 * 8]; 10 * 8]; // 4x10 sprites, 8x8 pixels each
            let obj_size = state.ppu.obj_size(); // false = 8x8, true = 8x16

            // Render all 40 sprites (OAM entries)
            for sprite_idx in 0..40 {
                let sprite_x = sprite_idx % 4;
                let sprite_y = sprite_idx / 4;

                if sprite_y >= 10 {
                    break; // Only display first 40 sprites in 4x10 grid
                }

                let oam_entry = &state.ppu.oam[sprite_idx];
                let tile_index = oam_entry.tile_index;
                let x_flip = oam_entry.x_flip();
                let y_flip = oam_entry.y_flip();
                let use_obp1 = oam_entry.dmg_palette();

                // Select the appropriate palette
                let palette = if use_obp1 {
                    state.ppu.obp1
                } else {
                    state.ppu.obp0
                };

                // For 8x16 sprites, we need to handle two tiles
                let tiles_to_render = if obj_size { 2 } else { 1 };

                for tile_offset in 0..tiles_to_render {
                    // Calculate actual tile index for 8x16 mode
                    let actual_tile_index = if obj_size {
                        // In 8x16 mode, bit 0 of tile index is ignored
                        // Top tile is tile_index & 0xFE, bottom tile is tile_index | 0x01
                        if tile_offset == 0 {
                            (tile_index & 0xFE) as usize
                        } else {
                            (tile_index | 0x01) as usize
                        }
                    } else {
                        tile_index as usize
                    };

                    // Get tile data
                    for line in 0..8 {
                        // Apply Y flip if needed
                        let actual_line = if y_flip {
                            if obj_size {
                                // For 8x16 sprites, flip across the entire 16 pixel height
                                let total_line = tile_offset * 8 + line;
                                let flipped_total = 15 - total_line;
                                flipped_total % 8
                            } else {
                                7 - line
                            }
                        } else {
                            line
                        };

                        // For 8x16 sprites with Y flip, we also need to swap the tiles
                        let tile_to_use = if obj_size && y_flip {
                            // When Y-flipped, the bottom tile becomes the top tile
                            if tile_offset == 0 {
                                (tile_index | 0x01) as usize
                            } else {
                                (tile_index & 0xFE) as usize
                            }
                        } else {
                            actual_tile_index
                        };

                        let byte1 = state.ppu.vram[tile_to_use][actual_line * 2];
                        let byte2 = state.ppu.vram[tile_to_use][actual_line * 2 + 1];

                        for pixel in 0..8 {
                            // Apply X flip if needed
                            let bit = if x_flip { pixel } else { 7 - pixel };
                            let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);

                            // Apply palette (note: color 0 is transparent for sprites)
                            let color = if color_num == 0 {
                                0 // Keep as transparent (we'll render as lightest color)
                            } else {
                                (palette >> (color_num * 2)) & 0b11
                            };

                            let x = sprite_x * 8 + pixel;
                            let y = sprite_y * 8
                                + (if obj_size {
                                    tile_offset * 8 + line
                                } else {
                                    line
                                });

                            // Only render if within bounds (for 8x16 sprites)
                            if y < 10 * 8 {
                                data[y][x] = color;
                            }
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
                for y in 0..(10 * 8) {
                    for x in 0..(4 * 8) {
                        let color_index = sprite_data[y][x] as usize;
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
                None, // Scale to fill entire window
            )
            .map_err(|e| e.to_string())?;
        window.canvas.present();

        Ok(())
    }
}
