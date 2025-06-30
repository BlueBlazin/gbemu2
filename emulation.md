- What we want to do is run the gameboy for 69905 cycles every 1/60 seconds.
- We do this by calling the `driver.run_for(MAX_CYCLES)` method.
- At this point there are two important things we need to handle.
  - First, the `MAX_CYCLES = 69905` cycles can (and should) finish running much earlier than 1/60 seconds. For the remainder of the duration we need to wait.
  - Second, `run_for` can return after finishing fewer cycles than `MAX_CYCLES` if another event happens. An example of such an event is what I've named `VBlank` in the enum but this basically is an event indicating that the emulated LCD screen needs to be redrawn with the updated pixel values.
  - In the case of such an early return, we then need to call `run_for` again but this time with the remaining cycles as the target. We keep doing this until the `MaxCycles` event is returned by `run_for` (and it will also have 0 remaining cycles at this point).

Some important code files are:

- src/driver.rs which defines the custom async-await based driver, the `run_for` method, the `ExecutionResult` struct and the `Event` enum.
- src/emulation_runner.rs which calls `driver.run_for`.
- src/renderer.rs which contains SDL2 code.
