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

// Pixel FIFO data structures
#[derive(Debug, Clone, Copy)]
pub struct PixelData {
    pub color: u8,           // 0-3
    pub palette: bool,       // Object palette (OBP0=false, OBP1=true)
    pub bg_priority: bool,   // Background priority flag
    pub object_priority: u8, // For object priority sorting (lower = higher priority)
}

impl PixelData {
    pub fn new() -> Self {
        Self {
            color: 0,
            palette: false,
            bg_priority: false,
            object_priority: 255,
        }
    }

    pub fn bg_pixel(color: u8) -> Self {
        Self {
            color,
            palette: false,
            bg_priority: false,
            object_priority: 255,
        }
    }

    pub fn obj_pixel(color: u8, palette: bool, bg_priority: bool, priority: u8) -> Self {
        Self {
            color,
            palette,
            bg_priority,
            object_priority: priority,
        }
    }
}

#[derive(Debug)]
pub struct PixelFifo {
    pub pixels: Vec<PixelData>,
}

impl PixelFifo {
    pub fn new() -> Self {
        Self {
            pixels: Vec::with_capacity(16),
        }
    }

    pub fn push(&mut self, pixel: PixelData) {
        self.pixels.push(pixel);
    }

    pub fn pop(&mut self) -> Option<PixelData> {
        if self.pixels.is_empty() {
            None
        } else {
            Some(self.pixels.remove(0))
        }
    }

    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }

    pub fn clear(&mut self) {
        self.pixels.clear();
    }
}

// Fetcher state machine
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FetcherState {
    GetTileMap,
    GetTileDataLow,
    GetTileDataHigh,
    Push,
}

#[derive(Debug)]
pub struct PixelFetcher {
    pub state: FetcherState,
    pub x: u8,
    pub y: u8,
    pub tile_index: u8,
    pub tile_data_low: u8,
    pub tile_data_high: u8,
    pub is_window: bool,
    pub window_line: u8,
}

impl PixelFetcher {
    pub fn new() -> Self {
        Self {
            state: FetcherState::GetTileMap,
            x: 0,
            y: 0,
            tile_index: 0,
            tile_data_low: 0,
            tile_data_high: 0,
            is_window: false,
            window_line: 0,
        }
    }

    pub fn reset(&mut self) {
        self.state = FetcherState::GetTileMap;
        self.tile_index = 0;
        self.tile_data_low = 0;
        self.tile_data_high = 0;
    }
}

// Object data for scanline
#[derive(Debug, Clone)]
pub struct ObjectData {
    pub x: u8,
    pub y: u8,
    pub tile_index: u8,
    pub attributes: u8,
    pub oam_index: u8,
}

// PPU internal state for rendering
#[derive(Debug)]
pub struct PpuRenderState {
    pub bg_fifo: PixelFifo,
    pub obj_fifo: PixelFifo,
    pub fetcher: PixelFetcher,
    pub pixel_x: u8,
    pub objects_on_line: Vec<ObjectData>,
    pub window_triggered: bool,
    pub scx_discard_count: u8,
    pub mode_3_penalty: u16,
}

impl PpuRenderState {
    pub fn new() -> Self {
        Self {
            bg_fifo: PixelFifo::new(),
            obj_fifo: PixelFifo::new(),
            fetcher: PixelFetcher::new(),
            pixel_x: 0,
            objects_on_line: Vec::new(),
            window_triggered: false,
            scx_discard_count: 0,
            mode_3_penalty: 0,
        }
    }

    pub fn reset_for_scanline(&mut self) {
        self.bg_fifo.clear();
        self.obj_fifo.clear();
        self.fetcher.reset();
        self.pixel_x = 0;
        self.objects_on_line.clear();
        self.window_triggered = false;
        self.scx_discard_count = 0;
        self.mode_3_penalty = 0;
    }
}

// Helper function to sleep for a number of dots and automatically increment the dots counter
async fn sleep_dots(dots: u16) {
    sleep(dots as usize).await;
    with_state_mut(|state| state.ppu.dots += dots);
}

// Main PPU async function
pub async fn ppu() {
    let mut render_state = PpuRenderState::new();

    loop {
        // Get current PPU state
        let (ly, _mode, lcdc) =
            with_state(|state| (state.ppu.ly, state.ppu.ppu_mode, state.ppu.lcdc));

        // Check if LCD is enabled
        if (lcdc & 0x80) == 0 {
            // LCD is disabled - reset everything
            with_state_mut(|state| {
                state.ppu.ly = 0;
                state.ppu.dots = 0;
                state.ppu.set_mode(0); // HBlank
                state.ppu.window_line = 0;
            });
            sleep_dots(1).await;
            continue;
        }

        if ly < VISIBLE_SCANLINES {
            // Visible scanlines (0-143)
            execute_visible_scanline(&mut render_state).await;
        } else if ly < SCANLINES_PER_FRAME - 1 {
            // VBlank scanlines (144-152)
            execute_vblank_scanline().await;
        } else {
            // Last scanline (153) - shorter scanline
            execute_last_scanline().await;

            // Emit VBlank event for host system
            emit_event(Event::VBlank);
        }
    }
}

async fn execute_visible_scanline(render_state: &mut PpuRenderState) {
    // Mode 2: OAM Scan (80 dots)
    with_state_mut(|state| state.ppu.set_mode(MODE_OAM_SCAN));

    // Trigger STAT interrupt if enabled
    let should_interrupt = with_state(|state| state.ppu.should_trigger_stat_interrupt());
    if should_interrupt {
        request_lcd_interrupt();
    }

    oam_scan(render_state).await;

    // Mode 3: Drawing (172-289 dots)
    with_state_mut(|state| state.ppu.set_mode(MODE_DRAWING));

    let mode_3_dots = draw_scanline(render_state).await;

    // Mode 0: HBlank
    with_state_mut(|state| state.ppu.set_mode(MODE_HBLANK));

    // Trigger STAT interrupt if enabled
    let should_interrupt = with_state(|state| state.ppu.should_trigger_stat_interrupt());
    if should_interrupt {
        request_lcd_interrupt();
    }

    let hblank_dots = DOTS_PER_SCANLINE - OAM_SCAN_DOTS - mode_3_dots;
    sleep_dots(hblank_dots).await;

    // Move to next scanline
    with_state_mut(|state| {
        state.ppu.ly += 1;
        state.ppu.dots = 0;
        state.ppu.update_lyc_ly_flag();

        // Check for LYC=LY interrupt
        if state.ppu.should_trigger_stat_interrupt() {
            request_lcd_interrupt();
        }
    });
}

async fn execute_vblank_scanline() {
    let ly = with_state(|state| state.ppu.ly);

    if ly == VISIBLE_SCANLINES {
        // First VBlank scanline - set mode and trigger interrupts
        with_state_mut(|state| state.ppu.set_mode(MODE_VBLANK));

        // Trigger VBlank interrupt
        request_vblank_interrupt();

        // Trigger STAT interrupt if VBlank interrupt is enabled
        let should_interrupt = with_state(|state| state.ppu.should_trigger_stat_interrupt());
        if should_interrupt {
            request_lcd_interrupt();
        }
    }

    // Sleep for full scanline
    sleep_dots(DOTS_PER_SCANLINE).await;

    // Move to next scanline
    with_state_mut(|state| {
        state.ppu.ly += 1;
        state.ppu.dots = 0;
        state.ppu.update_lyc_ly_flag();

        // Check for LYC=LY interrupt
        if state.ppu.should_trigger_stat_interrupt() {
            request_lcd_interrupt();
        }
    });
}

async fn execute_last_scanline() {
    // Scanline 153 is shorter - only 4 dots
    sleep_dots(4).await;

    // Reset to scanline 0
    with_state_mut(|state| {
        state.ppu.ly = 0;
        state.ppu.dots = 0;
        state.ppu.update_lyc_ly_flag();
        state.ppu.window_line = 0;

        // Check for LYC=LY interrupt
        if state.ppu.should_trigger_stat_interrupt() {
            request_lcd_interrupt();
        }
    });

    // Sleep for remainder of scanline
    sleep_dots(DOTS_PER_SCANLINE - 4).await;
}

async fn oam_scan(render_state: &mut PpuRenderState) {
    render_state.objects_on_line.clear();

    let (ly, obj_size, obj_enable) =
        with_state(|state| (state.ppu.ly, state.ppu.obj_size(), state.ppu.obj_enable()));

    if !obj_enable {
        sleep_dots(OAM_SCAN_DOTS).await;
        return;
    }

    let sprite_height = if obj_size { 16 } else { 8 };
    let mut objects_found = 0;

    // Scan OAM for objects on this scanline (takes 80 dots total)
    for i in 0..40 {
        // Each OAM entry takes 2 dots to scan
        sleep_dots(2).await;

        if objects_found >= 10 {
            continue; // Can't display more than 10 objects per scanline
        }

        let obj = with_state(|state| state.ppu.oam[i].clone());

        // Check if object is on this scanline
        let obj_y = obj.y as i16 - 16;
        let ly_i16 = ly as i16;

        if ly_i16 >= obj_y && ly_i16 < obj_y + sprite_height as i16 {
            render_state.objects_on_line.push(ObjectData {
                x: obj.x,
                y: obj.y,
                tile_index: obj.tile_index,
                attributes: obj.attributes,
                oam_index: i as u8,
            });
            objects_found += 1;
        }
    }

    // Sort objects by X coordinate (and OAM index for ties)
    render_state
        .objects_on_line
        .sort_by(|a, b| a.x.cmp(&b.x).then(a.oam_index.cmp(&b.oam_index)));
}

async fn draw_scanline(render_state: &mut PpuRenderState) -> u16 {
    render_state.reset_for_scanline();

    // Initialize fetcher for this scanline
    let (ly, scx) = with_state(|state| (state.ppu.ly, state.ppu.scx));

    render_state.fetcher.x = scx / 8;
    render_state.fetcher.y = ly;
    render_state.scx_discard_count = scx % 8;

    let mut dots_spent = 0;

    // Background scrolling penalty
    if render_state.scx_discard_count > 0 {
        sleep_dots(render_state.scx_discard_count as u16).await;
        dots_spent += render_state.scx_discard_count as u16;
    }

    // Initial tile fetch penalty (12 dots)
    sleep_dots(12).await;
    dots_spent += 12;

    while render_state.pixel_x < 160 {
        // Run fetcher step
        let fetcher_dots = run_fetcher_step(render_state).await;
        dots_spent += fetcher_dots;

        // Handle window trigger
        if should_trigger_window(render_state) {
            trigger_window(render_state).await;
            dots_spent += 6; // Window penalty
        }

        // Handle objects
        let obj_penalty = handle_objects_at_pixel(render_state).await;
        dots_spent += obj_penalty;

        // Render pixel if FIFOs have data
        if can_render_pixel(render_state) {
            render_pixel(render_state).await;
            render_state.pixel_x += 1;
            dots_spent += 1;
        }
    }

    // Minimum mode 3 time is 172 dots
    if dots_spent < 172 {
        sleep_dots(172 - dots_spent).await;
        dots_spent = 172;
    }

    dots_spent
}

fn should_trigger_window(render_state: &PpuRenderState) -> bool {
    if render_state.window_triggered {
        return false;
    }

    with_state(|state| {
        let window_visible = state.ppu.should_render_window();
        let window_x = state.ppu.get_window_x();

        window_visible && render_state.pixel_x as i16 >= window_x
    })
}

async fn trigger_window(render_state: &mut PpuRenderState) {
    render_state.window_triggered = true;
    render_state.bg_fifo.clear();
    render_state.fetcher.reset();
    render_state.fetcher.is_window = true;

    with_state_mut(|state| {
        render_state.fetcher.window_line = state.ppu.window_line;
        state.ppu.window_line += 1;
    });

    sleep_dots(6).await; // Window trigger penalty
}

async fn run_fetcher_step(render_state: &mut PpuRenderState) -> u16 {
    match render_state.fetcher.state {
        FetcherState::GetTileMap => {
            get_tile_map(render_state).await;
            render_state.fetcher.state = FetcherState::GetTileDataLow;
            2 // VRAM read takes 2 dots
        }
        FetcherState::GetTileDataLow => {
            get_tile_data_low(render_state).await;
            render_state.fetcher.state = FetcherState::GetTileDataHigh;
            2 // VRAM read takes 2 dots
        }
        FetcherState::GetTileDataHigh => {
            get_tile_data_high(render_state).await;
            render_state.fetcher.state = FetcherState::Push;
            2 // VRAM read takes 2 dots
        }
        FetcherState::Push => {
            if try_push_pixels(render_state) {
                // Successfully pushed, advance fetcher
                render_state.fetcher.x = (render_state.fetcher.x + 1) & 0x1F;
                render_state.fetcher.state = FetcherState::GetTileMap;
            }
            1 // Push attempt takes 1 dot
        }
    }
}

async fn get_tile_map(render_state: &mut PpuRenderState) {
    let tile_index = with_state(|state| {
        if render_state.fetcher.is_window {
            // Window tile map
            let map_base = state.ppu.get_window_tile_map_base();
            let tile_x = render_state.fetcher.x as u16;
            let tile_y = (render_state.fetcher.window_line / 8) as u16;
            let tile_addr = map_base + tile_y * 32 + tile_x;

            if tile_addr >= 0x9800 && tile_addr < 0x9C00 {
                state.ppu.bg_tile_map_0[(tile_addr - 0x9800) as usize]
            } else if tile_addr >= 0x9C00 && tile_addr < 0xA000 {
                state.ppu.bg_tile_map_1[(tile_addr - 0x9C00) as usize]
            } else {
                0xFF
            }
        } else {
            // Background tile map
            let map_base = state.ppu.get_bg_tile_map_base();
            let tile_x = render_state.fetcher.x as u16;
            let tile_y = ((state.ppu.ly.wrapping_add(state.ppu.scy)) / 8) as u16;
            let tile_addr = map_base + (tile_y & 31) * 32 + (tile_x & 31);

            if tile_addr >= 0x9800 && tile_addr < 0x9C00 {
                state.ppu.bg_tile_map_0[(tile_addr - 0x9800) as usize]
            } else if tile_addr >= 0x9C00 && tile_addr < 0xA000 {
                state.ppu.bg_tile_map_1[(tile_addr - 0x9C00) as usize]
            } else {
                0xFF
            }
        }
    });

    render_state.fetcher.tile_index = tile_index;
    sleep_dots(2).await;
}

async fn get_tile_data_low(render_state: &mut PpuRenderState) {
    let tile_data = with_state(|state| {
        let tile_addr = if render_state.fetcher.is_window {
            state.ppu.get_tile_address(render_state.fetcher.tile_index)
        } else {
            state.ppu.get_tile_address(render_state.fetcher.tile_index)
        };

        let line = if render_state.fetcher.is_window {
            render_state.fetcher.window_line % 8
        } else {
            (state.ppu.ly.wrapping_add(state.ppu.scy)) % 8
        };

        let tile_index = ((tile_addr - 0x8000) / 16) as usize;
        let byte_offset = (line * 2) as usize;

        if tile_index < 384 && byte_offset < 16 {
            state.ppu.vram[tile_index][byte_offset]
        } else {
            0xFF
        }
    });

    render_state.fetcher.tile_data_low = tile_data;
    sleep_dots(2).await;
}

async fn get_tile_data_high(render_state: &mut PpuRenderState) {
    let tile_data = with_state(|state| {
        let tile_addr = if render_state.fetcher.is_window {
            state.ppu.get_tile_address(render_state.fetcher.tile_index)
        } else {
            state.ppu.get_tile_address(render_state.fetcher.tile_index)
        };

        let line = if render_state.fetcher.is_window {
            render_state.fetcher.window_line % 8
        } else {
            (state.ppu.ly.wrapping_add(state.ppu.scy)) % 8
        };

        let tile_index = ((tile_addr - 0x8000) / 16) as usize;
        let byte_offset = (line * 2 + 1) as usize;

        if tile_index < 384 && byte_offset < 16 {
            state.ppu.vram[tile_index][byte_offset]
        } else {
            0xFF
        }
    });

    render_state.fetcher.tile_data_high = tile_data;
    sleep_dots(2).await;
}

fn try_push_pixels(render_state: &mut PpuRenderState) -> bool {
    // Only push if background FIFO is empty
    if !render_state.bg_fifo.is_empty() {
        return false;
    }

    // Extract 8 pixels from tile data and push to FIFO
    let low = render_state.fetcher.tile_data_low;
    let high = render_state.fetcher.tile_data_high;

    for bit in 0..8 {
        let pixel_bit = 7 - bit;
        let color = ((high >> pixel_bit) & 1) << 1 | ((low >> pixel_bit) & 1);
        render_state.bg_fifo.push(PixelData::bg_pixel(color));
    }

    true
}

async fn handle_objects_at_pixel(render_state: &mut PpuRenderState) -> u16 {
    let mut penalty = 0;

    // Find first object at current pixel
    let mut target_obj: Option<ObjectData> = None;
    for obj in &render_state.objects_on_line {
        let obj_x = obj.x as i16 - 8;
        if obj_x <= render_state.pixel_x as i16 && obj_x + 8 > render_state.pixel_x as i16 {
            target_obj = Some(obj.clone());
            break; // Only handle one object per pixel
        }
    }

    // Handle the object if found
    if let Some(obj) = target_obj {
        penalty += fetch_object_pixels(render_state, &obj).await;
    }

    penalty
}

async fn fetch_object_pixels(render_state: &mut PpuRenderState, obj: &ObjectData) -> u16 {
    let mut penalty = 6; // Base object penalty

    // Special case: object at X=0
    if obj.x == 0 {
        sleep_dots(11).await;
        return 11;
    }

    // Calculate object tile penalty based on background tile alignment
    let scx = with_state(|state| state.ppu.scx);
    let tile_pixel = (render_state.pixel_x + scx) % 8;
    let remaining_pixels = 8 - tile_pixel;

    if remaining_pixels > 2 {
        penalty += remaining_pixels as u16 - 2;
    }

    // Fetch object tile data
    let (obj_size, ly) = with_state(|state| (state.ppu.obj_size(), state.ppu.ly));
    let sprite_height = if obj_size { 16 } else { 8 };

    let obj_y = obj.y as i16 - 16;
    let sprite_line = ly as i16 - obj_y;

    let actual_line = if (obj.attributes & 0x40) != 0 {
        // Y flip
        sprite_height - 1 - sprite_line as u8
    } else {
        sprite_line as u8
    };

    let tile_index = if obj_size {
        // 8x16 mode
        if actual_line < 8 {
            obj.tile_index & 0xFE
        } else {
            obj.tile_index | 0x01
        }
    } else {
        obj.tile_index
    };

    let tile_line = actual_line % 8;

    let (tile_data_low, tile_data_high) = with_state(|state| {
        let tile_addr = 0x8000 + (tile_index as u16 * 16);
        let tile_idx = ((tile_addr - 0x8000) / 16) as usize;
        let byte_offset = (tile_line * 2) as usize;

        if tile_idx < 384 && byte_offset < 15 {
            (
                state.ppu.vram[tile_idx][byte_offset],
                state.ppu.vram[tile_idx][byte_offset + 1],
            )
        } else {
            (0xFF, 0xFF)
        }
    });

    // Fill object FIFO if needed
    while render_state.obj_fifo.len() < 8 {
        render_state.obj_fifo.push(PixelData::new());
    }

    // Mix object pixels into object FIFO
    let obj_x = obj.x as i16 - 8;
    let start_pixel = if obj_x < render_state.pixel_x as i16 {
        (render_state.pixel_x as i16 - obj_x) as usize
    } else {
        0
    };

    for i in start_pixel..8 {
        if (obj_x + i as i16) >= render_state.pixel_x as i16
            && (obj_x + i as i16)
                < (render_state.pixel_x as i16 + render_state.obj_fifo.len() as i16)
        {
            let pixel_bit = if (obj.attributes & 0x20) != 0 {
                // X flip
                i
            } else {
                7 - i
            };

            let color =
                ((tile_data_high >> pixel_bit) & 1) << 1 | ((tile_data_low >> pixel_bit) & 1);

            if color != 0 {
                // Only non-transparent pixels
                let fifo_index = (obj_x + i as i16 - render_state.pixel_x as i16) as usize;
                if fifo_index < render_state.obj_fifo.len() {
                    let current_pixel = &render_state.obj_fifo.pixels[fifo_index];

                    // Replace if current pixel is transparent or has lower priority
                    if current_pixel.color == 0 || obj.oam_index < current_pixel.object_priority {
                        render_state.obj_fifo.pixels[fifo_index] = PixelData::obj_pixel(
                            color,
                            (obj.attributes & 0x10) != 0, // Palette
                            (obj.attributes & 0x80) != 0, // BG priority
                            obj.oam_index,
                        );
                    }
                }
            }
        }
    }

    sleep_dots(penalty).await;
    penalty
}

fn can_render_pixel(render_state: &PpuRenderState) -> bool {
    !render_state.bg_fifo.is_empty() && render_state.pixel_x < 160
}

async fn render_pixel(render_state: &mut PpuRenderState) {
    // Pop pixels from both FIFOs
    let bg_pixel = render_state.bg_fifo.pop().unwrap_or(PixelData::bg_pixel(0));
    let obj_pixel = render_state.obj_fifo.pop().unwrap_or(PixelData::new());

    // Handle SCX discard
    if render_state.scx_discard_count > 0 {
        render_state.scx_discard_count -= 1;
        return; // Don't render this pixel
    }

    let _final_color = with_state_mut(|state| {
        // Determine final pixel color
        let (_lcdc, bg_enable, obj_enable) = (
            state.ppu.lcdc,
            state.ppu.bg_window_enable(),
            state.ppu.obj_enable(),
        );

        let mut final_color = 0;

        if bg_enable {
            final_color = state.ppu.get_bg_color(bg_pixel.color);
        }

        // Object priority logic
        if obj_enable && obj_pixel.color != 0 {
            let bg_color_zero = bg_pixel.color == 0;
            let obj_has_priority = !obj_pixel.bg_priority || bg_color_zero;

            if obj_has_priority {
                final_color = state.ppu.get_obj_color(obj_pixel.palette, obj_pixel.color);
            }
        }

        // Set pixel in framebuffer
        state.ppu.set_pixel(
            render_state.pixel_x as usize,
            state.ppu.ly as usize,
            final_color,
        );

        final_color
    });

    sleep_dots(1).await;
}
