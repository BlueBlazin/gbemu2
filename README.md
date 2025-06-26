# GBEmu 2

A gameboy emulator written in Rust.

## Details

Cycle accurate gameboy (DMG) emulator written as a proof-of-concept for my bespoke state machine implementation using (abusing) Rust's async-await.

The code for the async Driver is in src/driver.rs and an example of it is as follows:

```rs
async fn cpu() {
    sleep(3).await;
    println!("CPU: 1");
    sleep(3).await;
    println!("CPU: 2");
    sleep(2).await;
    println!("CPU: 3");
}

async fn gpu() {
    sleep(4).await;
    println!("GPU: 1");
    sleep(1).await;
    println!("GPU: 2");
    sleep(1).await;
    println!("GPU: 3");
}

async fn apu() {
    sleep(3).await;
    println!("APU: 1");
    sleep(2).await;
    println!("APU: 2");
    sleep(4).await;
    println!("APU: 3");
}

fn main() {
    let mut driver = Driver::new();

    driver.spawn(cpu());
    driver.spawn(gpu());
    driver.spawn(apu());

    // Run till completion.
    driver.run();
}
```

The code uses a simple function based approach rather than using struct impls. So all state (including CPU registers, RAM, ROM, etc.) is a state struct and other components use async functions to read/write it.
