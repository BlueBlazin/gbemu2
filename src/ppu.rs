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

    // LCD framebuffer - 160x144 pixels, 4 colors (0-3)
    pub framebuffer: [[u8; 160]; 144],

    // Internal PPU state
    pub ppu_mode: u8,         // Current PPU mode (0-3)
    pub dots: u16,            // Current dot/cycle within scanline
    pub window_line: u8,      // Internal window line counter
    pub frame_complete: bool, // Flag for frame completion
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
            framebuffer: [[0; 160]; 144], // Initialize to color 0 (white)
            ppu_mode: 2, // Start in mode 2 (OAM scan)
            dots: 0,
            window_line: 0,
            frame_complete: false,
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

    // Mark frame as complete
    with_state_mut(|state| state.ppu.frame_complete = true);
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

    // TODO: Trigger VBlank interrupt if enabled

    // Wait for full scanline duration
    sleep(DOTS_PER_SCANLINE as usize).await;
}

async fn execute_oam_scan() {
    // Set mode to OAM scan
    with_state_mut(|state| {
        state.ppu.set_mode(MODE_OAM_SCAN);
        state.ppu.update_lyc_ly_flag();
    });

    // TODO: Trigger Mode 2 interrupt if enabled

    // Scan OAM for objects on current scanline
    let selected_objects = scan_oam_for_objects().await;

    // TODO: Store selected objects for use in drawing mode

    // Wait for OAM scan duration
    sleep(OAM_SCAN_DOTS as usize).await;
}

async fn execute_drawing() {
    // Set mode to drawing
    with_state_mut(|state| state.ppu.set_mode(MODE_DRAWING));

    // TODO: Trigger Mode 3 interrupt if enabled

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

    // TODO: Trigger Mode 0 interrupt if enabled

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

    // TODO: Penalty 2: Window rendering (6 dots when window starts)
    // TODO: Penalty 3: Object rendering (6-11 dots per object)

    // Ensure we don't exceed maximum
    dots.min(DRAWING_MAX_DOTS)
}

async fn render_scanline() {
    // TODO: Implement pixel FIFO rendering
    // This is a placeholder for the complex pixel FIFO system
    
    // For now, just simulate the basic rendering process
    let ly = with_state(|state| state.ppu.ly);
    let bg_enabled = with_state(|state| state.ppu.bg_window_enable());
    let window_enabled = with_state(|state| state.ppu.should_render_window());
    let obj_enabled = with_state(|state| state.ppu.obj_enable());

    // TODO: Initialize background fetcher
    if bg_enabled {
        render_background_line().await;
    }

    // TODO: Handle window rendering
    if window_enabled {
        render_window_line().await;
    }

    // TODO: Handle object rendering  
    if obj_enabled {
        render_objects_line().await;
    }

    // TODO: Mix pixels and output to framebuffer
}

async fn render_background_line() {
    // TODO: Implement background tile fetching and pixel generation
    // This involves:
    // 1. Calculate tile map address based on SCY/SCX
    // 2. Fetch tile indices from tile map
    // 3. Fetch tile data from tile data area
    // 4. Generate pixels with palette application
    
    // Placeholder: simulate fetching 160 pixels worth of tiles
    for pixel in 0..160 {
        // TODO: Implement actual background pixel fetching
        sleep(1).await; // Simulate one dot per pixel (minimum)
    }
}

async fn render_window_line() {
    // TODO: Implement window tile fetching
    // Similar to background but uses window tile map and WX/WY positioning
    
    // Check if window should be rendered on this scanline
    let should_render = with_state(|state| state.ppu.should_render_window());
    if !should_render {
        return;
    }

    // TODO: Implement window pixel generation
    // Increment internal window line counter
    with_state_mut(|state| state.ppu.window_line += 1);
}

async fn render_objects_line() {
    // TODO: Implement object/sprite rendering
    // This involves:
    // 1. Use objects selected during OAM scan
    // 2. Fetch sprite tile data
    // 3. Apply palette and transparency
    // 4. Handle priority and overlap
    
    // Placeholder for object rendering
}

// Memory access functions for PPU (similar to CPU pattern)
async fn ppu_vram_read(address: u16) -> u8 {
    sleep(4).await; // VRAM access takes 4 cycles
    with_state(|state| state.mmu.read(address))
}

async fn ppu_oam_read(address: u16) -> u8 {
    sleep(4).await; // OAM access takes 4 cycles  
    with_state(|state| state.mmu.read(address))
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
