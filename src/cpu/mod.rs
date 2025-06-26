use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};

async fn memory_read(address: u16) -> u8 {
    sleep(4).await; // Memory read takes 4 cycles
    with_state(|state| state.memory[address as usize])
}

async fn fetch() -> u8 {
    let pc = with_state(|state| state.cpu.pc);
    
    let instruction = memory_read(pc).await;
    
    // Increment PC after fetch
    with_state_mut(|state| {
        state.cpu.pc = pc.wrapping_add(1);
    });
    
    instruction
}

pub async fn cpu() {
    loop {
        let instruction = fetch().await;
        // For now, just print the fetched instruction
        println!("Fetched instruction: 0x{:02X}", instruction);
        
        // TODO: Decode and execute instruction
        sleep(1).await; // Minimal cycle for now
    }
}
