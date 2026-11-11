# PatchMatrix frame timing

Synthetic 200 × 200 matrix, scrolled into the middle; release (`cargo test --release --locked --test matrix matrix_frames -- --ignored --nocapture --test-threads=1`). Linux x86-64 host, 32 logical threads (`nproc`); `uptime` at 09:36:58 local, immediately after the measurement: load averages 2.56, 1.88, 1.76 (1/5/15 min). Fonts from the workspace's Aizome test font directory. Results: `runs/2026-11-11/patchbay/phase1/s1b/timing.log`.

| Interaction | Scale | p50 (ms) | p95 (ms) | p95 ≤ 4 ms? |
|---|---:|---:|---:|---|
| Hover move | 1× | 0.532 | 0.547 | Yes |
| Key focus move | 1× | 0.459 | 0.468 | Yes |
| One-notch diagonal scroll (24 pt per axis) | 1× | 1.321 | 1.345 | Yes |
| Full window redraw | 1× | 1.225 | 1.241 | Yes |
| Hover move | 2× | 0.900 | 0.931 | Yes |
| Key focus move | 2× | 0.692 | 0.710 | Yes |
| One-notch diagonal scroll (24 pt per axis) | 2× | 2.781 | 2.836 | Yes |
| Full window redraw | 2× | 2.646 | 2.736 | Yes |

80 samples per operation and scale, first 10 discarded; p50/p95 are the sorted sample at indices 50%/95%. Interactions use the Harness input path; timing encloses `render_damage` on damage collected by `take_damage` (or the full-window rectangle for redraw), including damage gathering, paint and rasterization into retained pixels. This is a frame-rasterization measurement, not the kit's complete event-to-frame latency budget: input dispatch, update/layout and compositor presentation are outside the timed interval. The test's window is the app's Harness window rather than a 2880 × 1800 display. No measured p95 exceeds 4 ms. Scrolling redraws the moving headers and body instead of pixel-shifting them; its larger damaged area accounts for its higher cost than a hover/focus band. Full redraw uses a single gathered rectangle, while scroll gathers multiple regions.

The ordinary full test suite (no golden updates), including exact-frame checks at 1× and 2× and committed golden comparisons, passed: `runs/2026-11-11/patchbay/phase1/s1b/full-test.log`.
