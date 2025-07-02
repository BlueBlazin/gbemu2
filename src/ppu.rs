use crate::driver::{Event, emit_event, sleep};
use crate::emulator::{with_state, with_state_mut};
use crate::interrupts::{request_lcd_interrupt, request_vblank_interrupt};

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

    // VRAM - Video RAM storage
    // Tile data: 384 tiles, 16 bytes each
    pub vram: [[u8; 16]; 384],

    // Background tile maps
    pub bg_tile_map_0: [u8; 0x400], // 32x32 tiles at 0x9800-0x9BFF
    pub bg_tile_map_1: [u8; 0x400], // 32x32 tiles at 0x9C00-0x9FFF

    // LCD framebuffer - 160x144 pixels, 4 colors (0-3)
    pub framebuffer: [[u8; 160]; 144],

    // Internal PPU state
    pub ppu_mode: u8,    // Current PPU mode (0-3)
    pub dots: u16,       // Current dot/cycle within scanline
    pub window_line: u8, // Internal window line counter

    // Pixel FIFO state (not public, managed internally)
    bg_fifo: PixelFifo,
    obj_fifo: PixelFifo,
    fetcher: PixelFetcher,
    pixel_x: u8,            // Current pixel X position (0-159)
    window_triggered: bool, // Whether window has been triggered this scanline
    scx_discard_count: u8,
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
            vram: [[0; 16]; 384],         // Initialize VRAM to 0
            bg_tile_map_0: [0; 0x400],    // Initialize tile map 0 to 0
            bg_tile_map_1: [0; 0x400],    // Initialize tile map 1 to 0
            framebuffer: [[0; 160]; 144], // Initialize to color 0 (white)
            ppu_mode: 2,                  // Start in mode 2 (OAM scan)
            dots: 0,
            window_line: 0,
            bg_fifo: PixelFifo::new(),
            obj_fifo: PixelFifo::new(),
            fetcher: PixelFetcher::new(),
            pixel_x: 0,
            window_triggered: false,
            scx_discard_count: 0,
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

    // Check if STAT interrupt should be triggered
    pub fn should_trigger_stat_interrupt(&self) -> bool {
        // Check LYC=LY condition
        if self.lyc_int_select() && self.lyc_equals_ly() {
            return true;
        }

        // Check mode interrupts
        match self.ppu_mode {
            0 if self.mode_0_int_select() => true, // HBlank
            1 if self.mode_1_int_select() => true, // VBlank
            2 if self.mode_2_int_select() => true, // OAM Scan
            _ => false,
        }
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

// PPU register addresses
pub const LCDC_REGISTER: u16 = 0xFF40; // LCD Control
pub const STAT_REGISTER: u16 = 0xFF41; // LCD Status
pub const SCY_REGISTER: u16 = 0xFF42; // Scroll Y
pub const SCX_REGISTER: u16 = 0xFF43; // Scroll X
pub const LY_REGISTER: u16 = 0xFF44; // LCD Y Coordinate
pub const LYC_REGISTER: u16 = 0xFF45; // LY Compare
pub const BGP_REGISTER: u16 = 0xFF47; // Background Palette
pub const OBP0_REGISTER: u16 = 0xFF48; // Object Palette 0
pub const OBP1_REGISTER: u16 = 0xFF49; // Object Palette 1
pub const WY_REGISTER: u16 = 0xFF4A; // Window Y Position
pub const WX_REGISTER: u16 = 0xFF4B; // Window X Position

// PPU timing constants
const DOTS_PER_SCANLINE: u16 = 456;
const SCANLINES_PER_FRAME: u8 = 154;
const VISIBLE_SCANLINES: u8 = 144;

// Mode timings
const OAM_SCAN_DOTS: u16 = 80;

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

    fn len(&self) -> usize {
        self.pixels.len()
    }

    fn clear(&mut self) {
        self.pixels.clear();
    }
}

// Pixel Fetcher states
#[derive(Debug, Clone, Copy, PartialEq)]
enum FetcherStep {
    GetTile,
    GetTileDataLow,
    GetTileDataHigh,
    Push,
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
            FetcherStep::GetTileDataHigh => FetcherStep::Push,
            FetcherStep::Push => FetcherStep::GetTile,
        };
    }
}

// Helper function to sleep for a number of dots and automatically increment the dots counter
async fn sleep_dots(dots: u16) {
    sleep(dots as usize).await;
    with_state_mut(|state| state.ppu.dots += dots);
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
        state.ppu.window_line = 0;
    });

    // Execute visible scanlines (0-143)
    for scanline in 0..VISIBLE_SCANLINES {
        let should_trigger_lyc = with_state_mut(|state| {
            state.ppu.ly = scanline;
            state.ppu.update_lyc_ly_flag();
            state.ppu.lyc_int_select() && state.ppu.lyc_equals_ly()
        });

        if should_trigger_lyc {
            request_lcd_interrupt();
        }

        execute_visible_scanline().await;
    }

    // Execute VBlank scanlines (144-153)
    for scanline in VISIBLE_SCANLINES..SCANLINES_PER_FRAME {
        let should_trigger_lyc = with_state_mut(|state| {
            state.ppu.ly = scanline;
            state.ppu.update_lyc_ly_flag();
            state.ppu.lyc_int_select() && state.ppu.lyc_equals_ly()
        });

        if should_trigger_lyc {
            request_lcd_interrupt();
        }

        execute_vblank_scanline().await;
    }
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

    // Increment window line counter if window was visible on this scanline
    with_state_mut(|state| {
        if state.ppu.window_triggered {
            state.ppu.window_line += 1;
        }
    });
}

async fn execute_vblank_scanline() {
    // Reset scanline dots
    with_state_mut(|state| state.ppu.dots = 0);

    // Check if this is the first VBlank scanline
    let is_first_vblank = with_state(|state| state.ppu.ly == VISIBLE_SCANLINES);

    // Set mode to VBlank
    let should_trigger_stat = with_state_mut(|state| {
        state.ppu.set_mode(MODE_VBLANK);
        state.ppu.update_lyc_ly_flag();
        state.ppu.should_trigger_stat_interrupt()
    });

    // Trigger VBlank interrupt only on first VBlank scanline (LY=144)
    if is_first_vblank {
        request_vblank_interrupt();

        // Also emit VBlank event for host system
        emit_event(Event::VBlank);
    }

    // Trigger STAT interrupt if conditions are met
    if should_trigger_stat {
        request_lcd_interrupt();
    }

    // Wait for full scanline duration
    sleep_dots(DOTS_PER_SCANLINE).await;
}

async fn execute_oam_scan() {
    // Set mode to OAM scan
    let should_trigger_stat = with_state_mut(|state| {
        state.ppu.set_mode(MODE_OAM_SCAN);
        state.ppu.update_lyc_ly_flag();
        state.ppu.should_trigger_stat_interrupt()
    });

    // Trigger STAT interrupt if conditions are met
    if should_trigger_stat {
        request_lcd_interrupt();
    }

    // Scan OAM for objects on current scanline
    let _selected_objects = scan_oam_for_objects().await;

    // TODO: Store selected objects for use in drawing mode

    // Wait for OAM scan duration
    sleep_dots(OAM_SCAN_DOTS).await;
}

async fn execute_drawing() {
    // Set mode to drawing
    with_state_mut(|state| {
        state.ppu.set_mode(MODE_DRAWING);
        // Note: No STAT interrupt is triggered for Mode 3
    });

    // Initialize pixel FIFO and render scanline
    render_scanline().await;
}

async fn execute_hblank() {
    // Set mode to HBlank
    let should_trigger_stat = with_state_mut(|state| {
        state.ppu.set_mode(MODE_HBLANK);
        state.ppu.should_trigger_stat_interrupt()
    });

    // Trigger STAT interrupt if conditions are met
    if should_trigger_stat {
        request_lcd_interrupt();
    }

    // Calculate remaining dots for HBlank
    let current_dots = with_state(|state| state.ppu.dots);
    let hblank_dots = DOTS_PER_SCANLINE - current_dots;

    // Wait for HBlank duration
    sleep_dots(hblank_dots).await;
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

    // No sorting - objects are selected in OAM order (first 10 matching)
    // Drawing priority is handled differently during rendering
    selected_objects
}

async fn render_scanline() {
    with_state_mut(|state| {
        state.ppu.bg_fifo.clear();
        state.ppu.obj_fifo.clear();
        state.ppu.fetcher.reset();
        state.ppu.pixel_x = 0;
        state.ppu.window_triggered = false;
        state.ppu.scx_discard_count = state.ppu.scx % 8;
    });

    // This loop now models the PPU ticking one dot at a time during Mode 3
    while with_state(|state| state.ppu.pixel_x < 160) {
        // A single dot passes. The renderer gets first priority.
        sleep_dots(1).await;

        // 1. Check for and handle window trigger. This can reset the fetcher.
        check_window_trigger().await;

        // 2. The fetcher works in the background to keep the FIFO fed.
        // It will only run if the FIFO has space for a full tile (8 pixels).
        if with_state(|state| state.ppu.bg_fifo.len() <= 8) {
            // A fetcher step takes 2 dots, so we tick it here.
            // Note: We are abstracting the fetcher's internal 2-dot-per-step clock.
            // A truly perfect model would have a separate fetcher clock, but this is
            // a very effective and common simplification.
            step_fetcher();
        }

        // 3. Try to shift a pixel out of the FIFO to the screen.
        render_pixel().await;
    }
}

fn step_fetcher() {
    let step = with_state(|state| state.ppu.fetcher.step);

    if step == FetcherStep::Push {
        let can_push = with_state(|state| state.ppu.bg_fifo.len() <= 8);
        if !can_push {
            return;
        }
    }

    match step {
        FetcherStep::GetTile => execute_get_tile(),
        FetcherStep::GetTileDataLow => execute_get_tile_data_low(),
        FetcherStep::GetTileDataHigh => execute_get_tile_data_high(),
        FetcherStep::Push => execute_push(),
    }

    with_state_mut(|state| state.ppu.fetcher.advance_step());
}

fn execute_get_tile() {
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
        let window_x = with_state(|state| state.ppu.fetcher.x);
        (window_x, window_line)
    } else {
        let bg_x = ((scx / 8) + with_state(|state| state.ppu.fetcher.x)) & 0x1F;
        let bg_y = ly.wrapping_add(scy);
        (bg_x, bg_y)
    };

    let tile_map_base = if is_window {
        with_state(|state| state.ppu.get_window_tile_map_base())
    } else {
        with_state(|state| state.ppu.get_bg_tile_map_base())
    };

    let tile_map_addr = tile_map_base + ((tile_y as u16 / 8) * 32) + (tile_x as u16);

    // Read tile index directly
    let tile_index = with_state(|state| state.mmu.read_vram(tile_map_addr, &state.ppu));

    with_state_mut(|state| {
        state.ppu.fetcher.tile_index = tile_index;
        state.ppu.fetcher.y = tile_y;
    });
}

// Make this function NON-ASYNC and remove the .await
fn execute_get_tile_data_low() {
    let (tile_index, tile_y) =
        with_state(|state| (state.ppu.fetcher.tile_index, state.ppu.fetcher.y));

    let tile_addr = with_state(|state| state.ppu.get_tile_address(tile_index));
    let tile_line = tile_y % 8;
    let data_addr = tile_addr + (tile_line as u16 * 2);

    let tile_data_low = with_state(|state| state.mmu.read_vram(data_addr, &state.ppu));

    with_state_mut(|state| state.ppu.fetcher.tile_data_low = tile_data_low);
}

// Make this function NON-ASYNC and remove the .await
fn execute_get_tile_data_high() {
    let (tile_index, tile_y) =
        with_state(|state| (state.ppu.fetcher.tile_index, state.ppu.fetcher.y));

    let tile_addr = with_state(|state| state.ppu.get_tile_address(tile_index));
    let tile_line = tile_y % 8;
    let data_addr = tile_addr + (tile_line as u16 * 2) + 1;

    let tile_data_high = with_state(|state| state.mmu.read_vram(data_addr, &state.ppu));

    with_state_mut(|state| state.ppu.fetcher.tile_data_high = tile_data_high);
}

// Make this function NON-ASYNC and return nothing
fn execute_push() {
    // The check to ensure we can push is now done in step_fetcher
    let (tile_data_low, tile_data_high) = with_state(|state| {
        (
            state.ppu.fetcher.tile_data_low,
            state.ppu.fetcher.tile_data_high,
        )
    });

    for bit_pos in 0..8 {
        let bit = 7 - bit_pos;
        let low_bit = (tile_data_low >> bit) & 1;
        let high_bit = (tile_data_high >> bit) & 1;
        let color = (high_bit << 1) | low_bit;

        let pixel = FifoPixel::new(color, false, false);

        with_state_mut(|state| state.ppu.bg_fifo.push(pixel));
    }

    with_state_mut(|state| state.ppu.fetcher.x = (state.ppu.fetcher.x + 1) & 0x1F);
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
        sleep_dots(6).await;
    }
}

async fn process_objects_at_pixel() {
    // TODO: Implement object processing
    // This is complex and involves:
    // - Finding objects at current pixel
    // - Fetching object tile data
    // - Mixing with OAM FIFO
    // - Handling penalties and priority
}

async fn render_pixel() {
    // If the FIFO is empty, we can't do anything. The pipeline is stalled.
    // Just return and wait for the fetcher to catch up on the next dot.
    if with_state(|state| state.ppu.bg_fifo.len() == 0) {
        return;
    }

    // A pixel is available! Pop it from the FIFO.
    let bg_pixel = with_state_mut(|state| state.ppu.bg_fifo.pop()).unwrap();

    // Check if we need to discard this pixel for SCX fine-scrolling.
    let should_discard = with_state_mut(|state| {
        if state.ppu.scx_discard_count > 0 {
            state.ppu.scx_discard_count -= 1;
            true
        } else {
            false
        }
    });

    if should_discard {
        return; // Discarded. Don't render and don't advance pixel_x.
    }

    // --- This pixel is NOT being discarded, so we render it ---

    // Process objects that might mix with this pixel
    // if with_state(|state| state.ppu.obj_enable()) {
    //     process_objects_at_pixel().await;
    // }
    // let obj_pixel = with_state_mut(|state| state.ppu.obj_fifo.pop());

    // For now, let's simplify and ignore objects to ensure the background is correct.
    let obj_pixel: Option<FifoPixel> = None;

    let (pixel_x, ly, bg_enabled) = with_state(|state| {
        (
            state.ppu.pixel_x,
            state.ppu.ly,
            state.ppu.bg_window_enable(),
        )
    });

    let final_color = if let Some(obj_pixel) = obj_pixel {
        // Mixing logic here...
        // For now, this branch is unused.
        0
    } else {
        // No object pixel, just use the background pixel.
        if bg_enabled {
            with_state(|state| state.ppu.get_bg_color(bg_pixel.color))
        } else {
            0 // Background disabled
        }
    };

    // Write the final pixel to the framebuffer
    with_state_mut(|state| {
        state
            .ppu
            .set_pixel(pixel_x as usize, ly as usize, final_color);
    });

    // IMPORTANT: Only advance the screen X-coordinate after a pixel has been successfully rendered.
    with_state_mut(|state| state.ppu.pixel_x += 1);
}
