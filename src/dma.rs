use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};

#[derive(Debug, Clone)]
pub struct DmaState {
    pub source_address: u16,   // Full 16-bit source address (0x0000-0xDFFF)
    pub bytes_transferred: u8, // Number of bytes transferred so far (0-160)
    pub active: bool,          // Whether DMA transfer is currently active
    pub delay_cycles: u8,      // Delay before transfer starts (2 M-cycles)
    pub request_pending: bool, // Whether a DMA request is pending
}

impl DmaState {
    pub fn new() -> Self {
        Self {
            source_address: 0x0000,
            bytes_transferred: 0,
            active: false,
            delay_cycles: 0,
            request_pending: false,
        }
    }

    pub fn request_transfer(&mut self, source_page: u8) {
        // Set source address from FF46 register value
        // Upper 8 bits come from register, lower 8 bits start at 0x00
        self.source_address = (source_page as u16) << 8;
        self.bytes_transferred = 0;
        self.request_pending = true;
        self.delay_cycles = 2; // 2 M-cycle delay before transfer starts
    }

    pub fn is_bus_restricted(&self) -> bool {
        self.active
    }
}

pub async fn dma_component() {
    loop {
        sleep(4).await; // Check every M-cycle (4 T-cycles)

        let should_start = with_state_mut(|state| {
            if state.dma.request_pending {
                if state.dma.delay_cycles > 0 {
                    state.dma.delay_cycles -= 1;
                    false
                } else {
                    // Start the transfer
                    state.dma.active = true;
                    state.dma.request_pending = false;
                    true
                }
            } else {
                false
            }
        });

        if should_start {
            // Perform the DMA transfer
            await_dma_transfer().await;
        }
    }
}

async fn await_dma_transfer() {
    // Transfer 160 bytes from source to OAM over 160 M-cycles
    for byte_index in 0..160 {
        sleep(4).await; // 1 M-cycle per byte (4 T-cycles)

        with_state_mut(|state| {
            let source_addr = state.dma.source_address.wrapping_add(byte_index as u16);
            let dest_addr = 0xFE00 + byte_index as u16;

            // Read from source address
            let byte_value = if source_addr <= 0xDFFF {
                // Valid source range (ROM/RAM)
                match source_addr {
                    // ROM areas (handled by cartridge)
                    0x0000..=0x7FFF | 0xA000..=0xBFFF => {
                        if let Some(ref cartridge) = state.mmu.cartridge {
                            cartridge.read(source_addr)
                        } else {
                            0xFF
                        }
                    }
                    // Video RAM
                    0x8000..=0x9FFF => state.mmu.vram[(source_addr & 0x1FFF) as usize],
                    // Work RAM
                    0xC000..=0xDFFF => state.mmu.wram[(source_addr & 0x1FFF) as usize],
                    _ => 0xFF,
                }
            } else {
                // Invalid source address
                0xFF
            };

            // Write to OAM
            state.mmu.oam[(dest_addr - 0xFE00) as usize] = byte_value;
            state.dma.bytes_transferred = byte_index + 1;
        });
    }

    // Transfer complete
    with_state_mut(|state| {
        state.dma.active = false;
        state.dma.bytes_transferred = 0;
    });
}
