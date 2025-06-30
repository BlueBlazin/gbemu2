# PPU problems

In `async fn execute_oam_scan()` we are not performing cycle accurate OAM scan but instead just running

```rs
// Wait for OAM scan duration and update dots
sleep(80).await;
```

This is wrong. We need to actually emulate the scan over the course of the 80 dots.

---

Are we applying the SCX penalty organically?

---
