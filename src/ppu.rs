#[derive(Debug, Clone, Copy)]
pub struct OamEntry {
    pub y: u8,          // Y position + 16
    pub x: u8,          // X position + 8
    pub tile_index: u8, // Tile index
    pub attributes: u8, // Attributes/flags
}

impl OamEntry {
    pub fn new() -> Self {
        Self {
            y: 0,
            x: 0,
            tile_index: 0,
            attributes: 0,
        }
    }

    // Attribute flag getters
    pub fn priority(&self) -> bool {
        (self.attributes & 0x80) != 0
    }

    pub fn y_flip(&self) -> bool {
        (self.attributes & 0x40) != 0
    }

    pub fn x_flip(&self) -> bool {
        (self.attributes & 0x20) != 0
    }

    pub fn dmg_palette(&self) -> bool {
        (self.attributes & 0x10) != 0
    }

    // Attribute flag setters
    pub fn set_priority(&mut self, value: bool) {
        if value {
            self.attributes |= 0x80;
        } else {
            self.attributes &= !0x80;
        }
    }

    pub fn set_y_flip(&mut self, value: bool) {
        if value {
            self.attributes |= 0x40;
        } else {
            self.attributes &= !0x40;
        }
    }

    pub fn set_x_flip(&mut self, value: bool) {
        if value {
            self.attributes |= 0x20;
        } else {
            self.attributes &= !0x20;
        }
    }

    pub fn set_dmg_palette(&mut self, value: bool) {
        if value {
            self.attributes |= 0x10;
        } else {
            self.attributes &= !0x10;
        }
    }
}

#[derive(Debug, Clone)]
pub struct PpuState {
    // LCD Control Register (FF40)
    pub lcdc: u8,

    // LCD Status Register (FF41)
    pub stat: u8,

    // Scrolling registers (FF42-FF43)
    pub scy: u8, // Background Y position
    pub scx: u8, // Background X position

    // Current scanline (FF44)
    pub ly: u8, // LCD Y coordinate

    // LY Compare (FF45)
    pub lyc: u8, // LY Compare

    // Window position (FF4A-FF4B)
    pub wy: u8, // Window Y position
    pub wx: u8, // Window X position + 7

    // Palettes (FF47-FF49)
    pub bgp: u8,  // Background palette
    pub obp0: u8, // Object palette 0
    pub obp1: u8, // Object palette 1

    // OAM - 40 objects, 4 bytes each
    pub oam: [OamEntry; 40],

    // VRAM - Video RAM (0x8000-0x97FF: tiles, 0x9800-0x9FFF: tile maps)
    pub vram: [[u8; 16]; 384], // 384 tiles, 16 bytes each (8x8 pixels, 2 bits per pixel)
    pub bg_tile_map_0: [u8; 0x400], // Background tile map 0 (0x9800-0x9BFF)
    pub bg_tile_map_1: [u8; 0x400], // Background tile map 1 (0x9C00-0x9FFF)

    // LCD framebuffer - 160x144 pixels, 4 colors (0-3)
    pub framebuffer: [[u8; 160]; 144],

    // Internal PPU state
    pub ppu_mode: u8,         // Current PPU mode (0-3)
    pub dots: u16,            // Current dot/cycle within scanline
    pub window_line: u8,      // Internal window line counter
    pub frame_complete: bool, // Flag for frame completion

    // Pixel FIFO state (not public, managed internally)
    bg_fifo: PixelFifo,
    obj_fifo: PixelFifo,
    fetcher: PixelFetcher,
    pixel_x: u8,                           // Current pixel X position (0-159)
    window_triggered: bool,                // Whether window has been triggered this scanline
    scx_penalty_applied: bool,             // Whether SCX penalty has been applied
    selected_objects: Vec<SelectedObject>, // Objects selected during OAM scan for current scanline
}

impl PpuState {
    pub fn new() -> Self {
        Self {
            lcdc: 0x91, // Default LCDC value at boot
            stat: 0x85, // Default STAT value
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            wy: 0,
            wx: 0,
            bgp: 0xFC,  // Default background palette
            obp0: 0xFF, // Default object palette 0
            obp1: 0xFF, // Default object palette 1
            oam: [OamEntry::new(); 40],
            vram: [[0; 16]; 384], // Initialize VRAM tiles
            bg_tile_map_0: [0; 0x400], // Initialize background tile map 0
            bg_tile_map_1: [0; 0x400], // Initialize background tile map 1
            framebuffer: [[0; 160]; 144], // Initialize to color 0 (white)
            ppu_mode: 2,                  // Start in mode 2 (OAM scan)
            dots: 0,
            window_line: 0,
            frame_complete: false,
            bg_fifo: PixelFifo::new(),
            obj_fifo: PixelFifo::new(),
            fetcher: PixelFetcher::new(),
            pixel_x: 0,
            window_triggered: false,
            scx_penalty_applied: false,
            selected_objects: Vec::new(),
        }
    }

    // LCDC flag getters (FF40)
    pub fn lcd_enable(&self) -> bool {
        (self.lcdc & 0x80) != 0
    }

    pub fn window_tile_map_area(&self) -> bool {
        (self.lcdc & 0x40) != 0 // 0 = 9800-9BFF, 1 = 9C00-9FFF
    }

    pub fn window_enable(&self) -> bool {
        (self.lcdc & 0x20) != 0
    }

    pub fn bg_window_tile_data_area(&self) -> bool {
        (self.lcdc & 0x10) != 0 // 0 = 8800-97FF, 1 = 8000-8FFF
    }

    pub fn bg_tile_map_area(&self) -> bool {
        (self.lcdc & 0x08) != 0 // 0 = 9800-9BFF, 1 = 9C00-9FFF
    }

    pub fn obj_size(&self) -> bool {
        (self.lcdc & 0x04) != 0 // 0 = 8x8, 1 = 8x16
    }

    pub fn obj_enable(&self) -> bool {
        (self.lcdc & 0x02) != 0
    }

    pub fn bg_window_enable(&self) -> bool {
        (self.lcdc & 0x01) != 0
    }

    // LCDC flag setters
    pub fn set_lcdc_bit(&mut self, bit: u8, value: bool) {
        if value {
            self.lcdc |= 1 << bit;
        } else {
            self.lcdc &= !(1 << bit);
        }
    }

    // STAT flag getters (FF41)
    pub fn lyc_int_select(&self) -> bool {
        (self.stat & 0x40) != 0
    }

    pub fn mode_2_int_select(&self) -> bool {
        (self.stat & 0x20) != 0
    }

    pub fn mode_1_int_select(&self) -> bool {
        (self.stat & 0x10) != 0
    }

    pub fn mode_0_int_select(&self) -> bool {
        (self.stat & 0x08) != 0
    }

    pub fn lyc_equals_ly(&self) -> bool {
        (self.stat & 0x04) != 0
    }

    // STAT flag setters
    pub fn set_stat_bit(&mut self, bit: u8, value: bool) {
        if value {
            self.stat |= 1 << bit;
        } else {
            self.stat &= !(1 << bit);
        }
    }

    // Update LYC=LY flag
    pub fn update_lyc_ly_flag(&mut self) {
        let lyc_ly = self.ly == self.lyc;
        self.set_stat_bit(2, lyc_ly);
    }

    // Set PPU mode and update STAT register
    pub fn set_mode(&mut self, mode: u8) {
        self.ppu_mode = mode;
        self.stat = (self.stat & 0xFC) | (mode & 0x03);
    }

    // Tile addressing helpers
    pub fn get_bg_tile_map_base(&self) -> u16 {
        if self.bg_tile_map_area() {
            0x9C00
        } else {
            0x9800
        }
    }

    pub fn get_window_tile_map_base(&self) -> u16 {
        if self.window_tile_map_area() {
            0x9C00
        } else {
            0x9800
        }
    }

    pub fn get_tile_data_base(&self) -> u16 {
        if self.bg_window_tile_data_area() {
            0x8000 // Unsigned addressing
        } else {
            0x9000 // Signed addressing
        }
    }

    pub fn uses_signed_tile_addressing(&self) -> bool {
        !self.bg_window_tile_data_area()
    }

    // Calculate tile address from tile index
    pub fn get_tile_address(&self, tile_index: u8) -> u16 {
        if self.uses_signed_tile_addressing() {
            // $8800 method: signed addressing
            let signed_index = tile_index as i8;
            (0x9000u16).wrapping_add((signed_index as i16 * 16) as u16)
        } else {
            // $8000 method: unsigned addressing
            0x8000 + (tile_index as u16 * 16)
        }
    }

    // Object tile address (always uses $8000 method)
    pub fn get_object_tile_address(&self, tile_index: u8) -> u16 {
        0x8000 + (tile_index as u16 * 16)
    }

    // Palette color lookup
    pub fn get_bg_color(&self, color_index: u8) -> u8 {
        (self.bgp >> (color_index * 2)) & 0x03
    }

    pub fn get_obj_color(&self, palette: bool, color_index: u8) -> u8 {
        if color_index == 0 {
            return 0; // Color 0 is transparent for objects
        }
        let palette_data = if palette { self.obp1 } else { self.obp0 };
        (palette_data >> (color_index * 2)) & 0x03
    }

    // Window visibility check
    pub fn is_window_visible(&self) -> bool {
        self.window_enable() && self.bg_window_enable() && self.wx <= 166 && self.wy <= 143
    }

    // Check if window should be rendered on current scanline
    pub fn should_render_window(&self) -> bool {
        self.is_window_visible() && self.ly >= self.wy
    }

    // Get effective window X position (WX - 7)
    pub fn get_window_x(&self) -> i16 {
        (self.wx as i16) - 7
    }

    // Framebuffer access methods
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        if x < 160 && y < 144 {
            self.framebuffer[y][x] = color & 0x03; // Ensure only 2 bits (0-3)
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> u8 {
        if x < 160 && y < 144 {
            self.framebuffer[y][x]
        } else {
            0 // Return white for out-of-bounds
        }
    }

    pub fn clear_framebuffer(&mut self) {
        self.framebuffer = [[0; 160]; 144];
    }

    // Get reference to framebuffer for external rendering
    pub fn get_framebuffer(&self) -> &[[u8; 160]; 144] {
        &self.framebuffer
    }
}

use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};

// PPU timing constants
const DOTS_PER_SCANLINE: u16 = 456;
const SCANLINES_PER_FRAME: u8 = 154;
const VISIBLE_SCANLINES: u8 = 144;
const VBLANK_SCANLINES: u8 = 10;

// Mode timings
const OAM_SCAN_DOTS: u16 = 80;
const DRAWING_MIN_DOTS: u16 = 172;
const DRAWING_MAX_DOTS: u16 = 289;
const HBLANK_MIN_DOTS: u16 = 87;
const HBLANK_MAX_DOTS: u16 = 204;
const VBLANK_DOTS: u16 = 4560;

// PPU Modes
const MODE_HBLANK: u8 = 0;
const MODE_VBLANK: u8 = 1;
const MODE_OAM_SCAN: u8 = 2;
const MODE_DRAWING: u8 = 3;

// Structure to hold objects found during OAM scan
#[derive(Debug, Clone, Copy)]
struct SelectedObject {
    oam_index: u8,
    x: u8,
    y: u8,
    tile_index: u8,
    attributes: u8,
}

// Pixel FIFO structures
#[derive(Debug, Clone, Copy)]
struct FifoPixel {
    color: u8,         // 0-3
    palette: bool,     // For objects: false = OBP0, true = OBP1
    bg_priority: bool, // OBJ-to-BG Priority bit
}

impl FifoPixel {
    fn new(color: u8, palette: bool, bg_priority: bool) -> Self {
        Self {
            color: color & 0x03,
            palette,
            bg_priority,
        }
    }

    fn transparent() -> Self {
        Self {
            color: 0,
            palette: false,
            bg_priority: false,
        }
    }
}

#[derive(Debug, Clone)]
struct PixelFifo {
    pixels: Vec<FifoPixel>,
}

impl PixelFifo {
    fn new() -> Self {
        Self {
            pixels: Vec::with_capacity(16),
        }
    }

    fn push(&mut self, pixel: FifoPixel) {
        if self.pixels.len() < 16 {
            self.pixels.push(pixel);
        }
    }

    fn pop(&mut self) -> Option<FifoPixel> {
        if !self.pixels.is_empty() {
            Some(self.pixels.remove(0))
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }

    fn len(&self) -> usize {
        self.pixels.len()
    }

    fn clear(&mut self) {
        self.pixels.clear();
    }

    fn fill_with_transparent(&mut self, count: usize) {
        for _ in 0..count.min(16 - self.pixels.len()) {
            self.pixels.push(FifoPixel::transparent());
        }
    }
}

// Pixel Fetcher states
#[derive(Debug, Clone, Copy, PartialEq)]
enum FetcherStep {
    GetTile = 1,
    GetTileDataLow = 2,
    GetTileDataHigh = 3,
    Sleep = 4,
    Push = 5,
}

#[derive(Debug, Clone)]
struct PixelFetcher {
    step: FetcherStep,
    x: u8,              // Fetcher X coordinate (0-31)
    y: u8,              // Fetcher Y coordinate (0-255)
    tile_index: u8,     // Current tile being fetched
    tile_data_low: u8,  // Low byte of tile data
    tile_data_high: u8, // High byte of tile data
    is_window: bool,    // Whether currently fetching window
}

impl PixelFetcher {
    fn new() -> Self {
        Self {
            step: FetcherStep::GetTile,
            x: 0,
            y: 0,
            tile_index: 0,
            tile_data_low: 0,
            tile_data_high: 0,
            is_window: false,
        }
    }

    fn reset(&mut self) {
        self.step = FetcherStep::GetTile;
    }

    fn advance_step(&mut self) {
        self.step = match self.step {
            FetcherStep::GetTile => FetcherStep::GetTileDataLow,
            FetcherStep::GetTileDataLow => FetcherStep::GetTileDataHigh,
            FetcherStep::GetTileDataHigh => FetcherStep::Sleep,
            FetcherStep::Sleep => FetcherStep::Push,
            FetcherStep::Push => FetcherStep::GetTile,
        };
    }
}

// Main PPU async function
pub async fn ppu() {
    loop {
        // Check if LCD is enabled
        if !with_state(|state| state.ppu.lcd_enable()) {
            // LCD is disabled - PPU is inactive
            sleep(1).await; // Wait one dot and check again
            continue;
        }

        // Execute one complete frame
        execute_frame().await;
    }
}

async fn execute_frame() {
    // Reset frame state
    with_state_mut(|state| {
        state.ppu.ly = 0;
        state.ppu.dots = 0;
        state.ppu.frame_complete = false;
        state.ppu.window_line = 0;
    });

    // Execute visible scanlines (0-143)
    for scanline in 0..VISIBLE_SCANLINES {
        with_state_mut(|state| state.ppu.ly = scanline);
        execute_visible_scanline().await;
    }

    // Execute VBlank scanlines (144-153)
    for scanline in VISIBLE_SCANLINES..SCANLINES_PER_FRAME {
        with_state_mut(|state| state.ppu.ly = scanline);
        execute_vblank_scanline().await;
    }

    // Mark frame as complete and emit VBlank event
    with_state_mut(|state| state.ppu.frame_complete = true);
    crate::driver::emit_event(crate::driver::Event::VBlank);
}

async fn execute_visible_scanline() {
    // Reset scanline state
    with_state_mut(|state| state.ppu.dots = 0);

    // Mode 2: OAM Scan (80 dots)
    execute_oam_scan().await;

    // Mode 3: Drawing (172-289 dots, variable based on penalties)
    execute_drawing().await;

    // Mode 0: HBlank (remaining dots until 456 total)
    execute_hblank().await;
}

async fn execute_vblank_scanline() {
    // Set mode to VBlank
    with_state_mut(|state| {
        state.ppu.set_mode(MODE_VBLANK);
        state.ppu.update_lyc_ly_flag();
    });

    // Trigger VBlank interrupt
    crate::interrupts::request_vblank_interrupt();

    // Wait for full scanline duration
    sleep(DOTS_PER_SCANLINE as usize).await;
}

async fn execute_oam_scan() {
    // Set mode to OAM scan
    with_state_mut(|state| {
        state.ppu.set_mode(MODE_OAM_SCAN);
        state.ppu.update_lyc_ly_flag();
    });

    // Trigger LCD interrupt if enabled for Mode 2
    let mode_2_int = with_state(|state| state.ppu.mode_2_int_select());
    if mode_2_int {
        crate::interrupts::request_lcd_interrupt();
    }

    // Scan OAM for objects on current scanline
    let selected_objects = scan_oam_for_objects().await;

    // Store selected objects for use in drawing mode
    with_state_mut(|state| {
        state.ppu.selected_objects = selected_objects;
    });

    // Wait for OAM scan duration
    sleep(OAM_SCAN_DOTS as usize).await;
}

async fn execute_drawing() {
    // Set mode to drawing
    with_state_mut(|state| state.ppu.set_mode(MODE_DRAWING));

    // Mode 3 doesn't trigger interrupts directly

    // Calculate drawing duration based on penalties
    let drawing_dots = calculate_drawing_duration().await;

    // TODO: Initialize pixel FIFO and render scanline
    render_scanline().await;

    // Wait for drawing duration
    sleep(drawing_dots as usize).await;
}

async fn execute_hblank() {
    // Set mode to HBlank
    with_state_mut(|state| state.ppu.set_mode(MODE_HBLANK));

    // Trigger LCD interrupt if enabled for Mode 0
    let mode_0_int = with_state(|state| state.ppu.mode_0_int_select());
    if mode_0_int {
        crate::interrupts::request_lcd_interrupt();
    }

    // Calculate remaining dots for HBlank
    let current_dots = with_state(|state| state.ppu.dots);
    let hblank_dots = DOTS_PER_SCANLINE - current_dots;

    // Wait for HBlank duration
    sleep(hblank_dots as usize).await;
}

async fn scan_oam_for_objects() -> Vec<SelectedObject> {
    let mut selected_objects = Vec::new();
    let ly = with_state(|state| state.ppu.ly);
    let obj_size = with_state(|state| state.ppu.obj_size());
    let obj_height = if obj_size { 16 } else { 8 };

    // Scan all 40 OAM entries
    for i in 0..40 {
        let oam_entry = with_state(|state| state.ppu.oam[i]);

        // Check if object is on current scanline
        let obj_y = oam_entry.y.wrapping_sub(16);
        if ly >= obj_y && ly < obj_y.wrapping_add(obj_height) {
            selected_objects.push(SelectedObject {
                oam_index: i as u8,
                x: oam_entry.x,
                y: oam_entry.y,
                tile_index: oam_entry.tile_index,
                attributes: oam_entry.attributes,
            });

            // Maximum 10 objects per scanline
            if selected_objects.len() >= 10 {
                break;
            }
        }
    }

    // Sort by X position for proper priority
    selected_objects.sort_by_key(|obj| obj.x);

    selected_objects
}

async fn calculate_drawing_duration() -> u16 {
    let mut dots = DRAWING_MIN_DOTS;

    // Penalty 1: Background scrolling (SCX % 8 dots)
    let scx_penalty = with_state(|state| (state.ppu.scx % 8) as u16);
    dots += scx_penalty;

    // Penalty 2: Window rendering (6 dots when window starts)
    let window_penalty = with_state(|state| {
        if state.ppu.should_render_window() {
            6 // Window incurs 6-dot penalty when it starts
        } else {
            0
        }
    });
    dots += window_penalty;

    // Penalty 3: Object rendering (6-11 dots per object)
    let object_penalty = with_state(|state| {
        let selected_objects = &state.ppu.selected_objects;
        calculate_object_penalties(selected_objects)
    });
    dots += object_penalty;

    // Ensure we don't exceed maximum
    dots.min(DRAWING_MAX_DOTS)
}

fn calculate_object_penalties(selected_objects: &[SelectedObject]) -> u16 {
    let mut penalty = 0;

    let scx = with_state(|state| state.ppu.scx);

    for obj in selected_objects {
        // Only the object's leftmost pixel matters, transparent or not
        let obj_x = obj.x.saturating_sub(8); // Convert from OAM coordinate to screen coordinate

        if obj_x == 0 && obj.x == 0 {
            // Special case: OBJ with OAM X position of 0 (completely off left side)
            penalty += 11;
        } else {
            // Determine the tile that the pixel is within (affected by horizontal scrolling)
            let _tile_offset = (obj_x + scx) / 8;
            let pixels_right_of_obj = 8 - ((obj_x + scx) % 8);

            // Subtract 2 and take max with 0 for waiting for BG fetch to finish
            let bg_fetch_penalty = pixels_right_of_obj.saturating_sub(2);

            // Flat 6-dot penalty for fetching the object's tile
            let fetch_penalty = 6;

            penalty += bg_fetch_penalty as u16 + fetch_penalty;
        }
    }

    penalty
}

async fn render_scanline() {
    // Initialize FIFO state for the scanline
    with_state_mut(|state| {
        state.ppu.bg_fifo.clear();
        state.ppu.obj_fifo.clear();
        state.ppu.fetcher.reset();
        state.ppu.pixel_x = 0;
        state.ppu.window_triggered = false;
        state.ppu.scx_penalty_applied = false;
        state.ppu.selected_objects.clear();
    });

    // Apply SCX penalty if needed
    apply_scx_penalty().await;

    // Main pixel rendering loop
    while with_state(|state| state.ppu.pixel_x < 160) {
        // Ensure we have enough pixels in background FIFO
        while with_state(|state| state.ppu.bg_fifo.len() < 8) {
            step_fetcher().await;
        }

        // Check for window trigger
        check_window_trigger().await;

        // Process objects if enabled
        if with_state(|state| state.ppu.obj_enable()) {
            process_objects_at_pixel().await;
        }

        // Render a pixel
        render_pixel().await;

        // Advance pixel position
        with_state_mut(|state| state.ppu.pixel_x += 1);
    }
}

async fn apply_scx_penalty() {
    let scx_penalty = with_state(|state| state.ppu.scx % 8);
    if scx_penalty > 0 {
        sleep(scx_penalty as usize).await;
        with_state_mut(|state| state.ppu.scx_penalty_applied = true);
    }
}

async fn step_fetcher() {
    let step = with_state(|state| state.ppu.fetcher.step);

    match step {
        FetcherStep::GetTile => {
            execute_get_tile().await;
            sleep(2).await; // Get Tile takes 2 dots
        }
        FetcherStep::GetTileDataLow => {
            execute_get_tile_data_low().await;
            sleep(2).await; // Get Tile Data Low takes 2 dots
        }
        FetcherStep::GetTileDataHigh => {
            execute_get_tile_data_high().await;
            sleep(2).await; // Get Tile Data High takes 2 dots
        }
        FetcherStep::Sleep => {
            sleep(2).await; // Sleep takes 2 dots
        }
        FetcherStep::Push => {
            // Push step is attempted every dot until it succeeds
            let success = execute_push().await;
            sleep(1).await; // Always takes at least 1 dot

            if !success {
                // Push failed, don't advance step
                return;
            }
        }
    }

    // Advance to next step
    with_state_mut(|state| state.ppu.fetcher.advance_step());
}

async fn execute_get_tile() {
    let (is_window, ly, scx, scy, window_line) = with_state(|state| {
        (
            state.ppu.fetcher.is_window,
            state.ppu.ly,
            state.ppu.scx,
            state.ppu.scy,
            state.ppu.window_line,
        )
    });

    let (tile_x, tile_y) = if is_window {
        // Window coordinates
        let window_x = with_state(|state| state.ppu.fetcher.x);
        (window_x, window_line)
    } else {
        // Background coordinates
        let bg_x = ((scx / 8) + with_state(|state| state.ppu.fetcher.x)) & 0x1F;
        let bg_y = (ly.wrapping_add(scy)) & 0xFF;
        (bg_x, bg_y)
    };

    // Determine which tile map to use
    let tile_map_base = if is_window {
        with_state(|state| state.ppu.get_window_tile_map_base())
    } else {
        with_state(|state| state.ppu.get_bg_tile_map_base())
    };

    // Calculate tile address
    let tile_addr = tile_map_base + ((tile_y as u16 / 8) * 32) + (tile_x as u16);

    // Read tile index
    let tile_index = if can_access_vram() {
        ppu_vram_read(tile_addr).await
    } else {
        0xFF
    };

    with_state_mut(|state| {
        state.ppu.fetcher.tile_index = tile_index;
        state.ppu.fetcher.x = tile_x;
        state.ppu.fetcher.y = tile_y;
    });
}

async fn execute_get_tile_data_low() {
    let (tile_index, tile_y) =
        with_state(|state| (state.ppu.fetcher.tile_index, state.ppu.fetcher.y));

    let tile_addr = with_state(|state| state.ppu.get_tile_address(tile_index));
    let tile_line = tile_y % 8;
    let data_addr = tile_addr + (tile_line as u16 * 2);

    let tile_data_low = if can_access_vram() {
        ppu_vram_read(data_addr).await
    } else {
        0xFF
    };

    with_state_mut(|state| state.ppu.fetcher.tile_data_low = tile_data_low);
}

async fn execute_get_tile_data_high() {
    let (tile_index, tile_y) =
        with_state(|state| (state.ppu.fetcher.tile_index, state.ppu.fetcher.y));

    let tile_addr = with_state(|state| state.ppu.get_tile_address(tile_index));
    let tile_line = tile_y % 8;
    let data_addr = tile_addr + (tile_line as u16 * 2) + 1;

    let tile_data_high = if can_access_vram() {
        ppu_vram_read(data_addr).await
    } else {
        0xFF
    };

    with_state_mut(|state| state.ppu.fetcher.tile_data_high = tile_data_high);
}

async fn execute_push() -> bool {
    // Only push if background FIFO is empty
    let can_push = with_state(|state| state.ppu.bg_fifo.is_empty());

    if !can_push {
        return false;
    }

    let (tile_data_low, tile_data_high) = with_state(|state| {
        (
            state.ppu.fetcher.tile_data_low,
            state.ppu.fetcher.tile_data_high,
        )
    });

    // Generate 8 pixels from tile data
    for bit_pos in 0..8 {
        let bit = 7 - bit_pos; // MSB first (unless flipped)
        let low_bit = (tile_data_low >> bit) & 1;
        let high_bit = (tile_data_high >> bit) & 1;
        let color = (high_bit << 1) | low_bit;

        let pixel = FifoPixel::new(color, false, false);

        with_state_mut(|state| state.ppu.bg_fifo.push(pixel));
    }

    // Advance fetcher X position
    with_state_mut(|state| state.ppu.fetcher.x = (state.ppu.fetcher.x + 1) & 0x1F);

    true
}

async fn check_window_trigger() {
    let (should_render_window, pixel_x, window_triggered, wx) = with_state(|state| {
        (
            state.ppu.should_render_window(),
            state.ppu.pixel_x,
            state.ppu.window_triggered,
            state.ppu.wx,
        )
    });

    if should_render_window && !window_triggered && pixel_x >= wx.saturating_sub(7) {
        // Trigger window - clear BG FIFO and reset fetcher
        with_state_mut(|state| {
            state.ppu.bg_fifo.clear();
            state.ppu.fetcher.reset();
            state.ppu.fetcher.is_window = true;
            state.ppu.window_triggered = true;
        });

        // 6-dot penalty for window
        sleep(6).await;
    }
}

async fn process_objects_at_pixel() {
    let pixel_x = with_state(|state| state.ppu.pixel_x);

    // Find objects at current pixel position
    let objects_at_pixel = with_state(|state| {
        state
            .ppu
            .selected_objects
            .iter()
            .filter(|obj| {
                let obj_x = obj.x.saturating_sub(8); // Convert OAM coordinate to screen coordinate
                let obj_end = obj_x + 8;
                pixel_x >= obj_x && pixel_x < obj_end
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    if objects_at_pixel.is_empty() {
        return;
    }

    // Process each object at this pixel (in priority order)
    for obj in objects_at_pixel {
        process_single_object(&obj).await;
    }
}

async fn process_single_object(obj: &SelectedObject) {
    let (_pixel_x, ly, scx, obj_size) = with_state(|state| {
        (
            state.ppu.pixel_x,
            state.ppu.ly,
            state.ppu.scx,
            state.ppu.obj_size(),
        )
    });

    // Advance fetcher one step until it's at step 5 or until the background FIFO is not empty
    while with_state(|state| {
        state.ppu.fetcher.step != FetcherStep::Push && state.ppu.bg_fifo.is_empty()
    }) {
        step_fetcher().await;
    }

    // Handle SCX penalty for objects at X coordinate 0
    let obj_x = obj.x.saturating_sub(8);
    if obj_x == 0 && scx & 7 > 0 {
        let penalty = scx & 7;
        sleep(penalty as usize).await;
    }

    // Advance fetcher two steps with penalties
    step_fetcher().await; // First advancement (1 dot)
    sleep(3).await; // Second advancement (3 dots)

    // Fetch object tile data
    let (tile_data_low, tile_data_high) = fetch_object_tile_data(obj, ly, obj_size).await;

    // Ensure OAM FIFO has at least 8 pixels
    while with_state(|state| state.ppu.obj_fifo.len() < 8) {
        with_state_mut(|state| state.ppu.obj_fifo.fill_with_transparent(1));
    }

    // Mix object pixels into OAM FIFO
    mix_object_pixels_into_fifo(obj, tile_data_low, tile_data_high, ly, obj_size).await;
}

async fn fetch_object_tile_data(obj: &SelectedObject, ly: u8, obj_size: bool) -> (u8, u8) {
    let obj_height = if obj_size { 16 } else { 8 };
    let obj_y = obj.y.saturating_sub(16);
    let tile_line = if obj_y <= ly { ly - obj_y } else { 0 };

    // Handle Y flip
    let actual_tile_line = if (obj.attributes & 0x40) != 0 {
        (obj_height - 1) - tile_line
    } else {
        tile_line
    };

    // Calculate tile index for 8x16 mode
    let tile_index = if obj_size {
        // 8x16 mode: use even tiles for top half, odd for bottom half
        if actual_tile_line < 8 {
            obj.tile_index & 0xFE // Even tile (top half)
        } else {
            obj.tile_index | 0x01 // Odd tile (bottom half)
        }
    } else {
        obj.tile_index
    };

    // Objects always use $8000 addressing
    let tile_addr = 0x8000 + (tile_index as u16 * 16);
    let line_in_tile = actual_tile_line % 8;
    let data_addr = tile_addr + (line_in_tile as u16 * 2);

    // Fetch tile data with cycle timing
    sleep(1).await; // Lower address fetch
    let tile_data_low = if can_access_vram() {
        ppu_vram_read(data_addr).await
    } else {
        0xFF
    };

    let tile_data_high = if can_access_vram() {
        ppu_vram_read(data_addr + 1).await
    } else {
        0xFF
    };

    sleep(1).await; // Exit object fetch penalty

    (tile_data_low, tile_data_high)
}

async fn mix_object_pixels_into_fifo(
    obj: &SelectedObject,
    tile_data_low: u8,
    tile_data_high: u8,
    _ly: u8,
    _obj_size: bool,
) {
    let pixel_x = with_state(|state| state.ppu.pixel_x);
    let obj_x = obj.x.saturating_sub(8);

    // Calculate which pixels of the tile to mix
    let start_bit = if pixel_x >= obj_x { pixel_x - obj_x } else { 0 };

    // Generate pixels for the object tile row
    for bit_pos in start_bit..8.min(start_bit + 8) {
        let bit = if (obj.attributes & 0x20) != 0 {
            // X flip: start from LSB
            bit_pos
        } else {
            // Normal: start from MSB
            7 - bit_pos
        };

        let low_bit = (tile_data_low >> bit) & 1;
        let high_bit = (tile_data_high >> bit) & 1;
        let color = (high_bit << 1) | low_bit;

        // Get palette selection
        let palette = (obj.attributes & 0x10) != 0; // DMG palette bit
        let bg_priority = (obj.attributes & 0x80) != 0; // Priority bit

        let obj_pixel = FifoPixel::new(color, palette, bg_priority);

        // Mix with existing pixel in OAM FIFO if conditions are met
        with_state_mut(|state| {
            if let Some(existing_pixel) = state.ppu.obj_fifo.pixels.get_mut(bit_pos as usize) {
                // Replace if target object pixel is not transparent and
                // (existing pixel is transparent OR existing pixel has higher priority)
                if color != 0 && (existing_pixel.color == 0 || !existing_pixel.bg_priority) {
                    *existing_pixel = obj_pixel;
                }
            }
        });
    }
}

async fn render_pixel() {
    let bg_pixel = with_state_mut(|state| state.ppu.bg_fifo.pop());
    let obj_pixel = with_state_mut(|state| state.ppu.obj_fifo.pop());

    let (pixel_x, ly, bg_enabled) = with_state(|state| {
        (
            state.ppu.pixel_x,
            state.ppu.ly,
            state.ppu.bg_window_enable(),
        )
    });

    // Skip rendering if we're past the visible area
    if pixel_x >= 160 {
        return;
    }

    let final_color = if let Some(bg_pixel) = bg_pixel {
        if let Some(obj_pixel) = obj_pixel {
            // Mix background and object pixels
            if obj_pixel.color != 0 {
                // Object pixel is not transparent
                // Object has priority unless bg has higher priority
                if obj_pixel.bg_priority && bg_pixel.color != 0 {
                    // Background has priority
                    with_state(|state| state.ppu.get_bg_color(bg_pixel.color))
                } else {
                    // Object has priority
                    with_state(|state| state.ppu.get_obj_color(obj_pixel.palette, obj_pixel.color))
                }
            } else {
                // Object is transparent, use background
                if bg_enabled {
                    with_state(|state| state.ppu.get_bg_color(bg_pixel.color))
                } else {
                    0 // Background disabled
                }
            }
        } else {
            // No object pixel, use background
            if bg_enabled {
                with_state(|state| state.ppu.get_bg_color(bg_pixel.color))
            } else {
                0 // Background disabled
            }
        }
    } else {
        0 // No pixels available
    };

    // Write pixel to framebuffer
    with_state_mut(|state| {
        state
            .ppu
            .set_pixel(pixel_x as usize, ly as usize, final_color);
    });
}

// Memory access functions for PPU - direct access to PPU state
async fn ppu_vram_read(address: u16) -> u8 {
    sleep(4).await; // VRAM access takes 4 cycles
    with_state(|state| {
        let vram_addr = (address & 0x1FFF) as usize;
        if vram_addr < 0x1800 {
            // Tile data area (0x8000-0x97FF)
            let tile_index = vram_addr / 16;
            let byte_in_tile = vram_addr % 16;
            state.ppu.vram[tile_index][byte_in_tile]
        } else {
            // Tile map areas (0x9800-0x9FFF)
            let map_addr = vram_addr - 0x1800;
            if map_addr < 0x400 {
                state.ppu.bg_tile_map_0[map_addr]
            } else {
                state.ppu.bg_tile_map_1[map_addr - 0x400]
            }
        }
    })
}

async fn ppu_oam_read(address: u16) -> u8 {
    sleep(4).await; // OAM access takes 4 cycles  
    with_state(|state| {
        let oam_index = ((address - 0xFE00) / 4) as usize;
        let byte_offset = (address - 0xFE00) % 4;
        if oam_index < 40 {
            match byte_offset {
                0 => state.ppu.oam[oam_index].y,
                1 => state.ppu.oam[oam_index].x,
                2 => state.ppu.oam[oam_index].tile_index,
                3 => state.ppu.oam[oam_index].attributes,
                _ => 0xFF,
            }
        } else {
            0xFF
        }
    })
}

// Helper function to check if PPU can access memory
fn can_access_vram() -> bool {
    with_state(|state| {
        let mode = state.ppu.ppu_mode;
        mode != MODE_DRAWING // VRAM accessible in modes 0, 1, 2 but not 3
    })
}

fn can_access_oam() -> bool {
    with_state(|state| {
        let mode = state.ppu.ppu_mode;
        mode == MODE_HBLANK || mode == MODE_VBLANK // OAM accessible in modes 0, 1 only
    })
}
