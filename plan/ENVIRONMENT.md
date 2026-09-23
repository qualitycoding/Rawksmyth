# Environment Pinning (plan/ENVIRONMENT.md)

## 1. Toolchain & Runtimes
- **Rust:** `1.83.0` (stable)
- **Cargo:** `1.83.0`
- **Godot Engine:** `4.3-stable` (standard edition, 64-bit)
- **GDExtension Binding:** `godot-rust` (`gdext` git commit pinned or latest 0.2.x release)
- **C/C++ Compiler:** Clang 17+ or GCC 13+ (for native Cubeb compilation via `cc-rs` / CMake)
- **CMake:** `3.28+`

## 2. Pinned Dependencies (Cargo.toml)
- `cubeb = "0.13.0"` (via `cubeb-rs`)
- `cpal = "0.15.3"` (fallback backend)
- `rtrb = "0.3.1"` (lock-free single-producer single-consumer ring buffer)
- `realfft = "3.3.0"` (high-performance real-valued FFT)
- `serde = { version = "1.0", features = ["derive"] }`
- `serde_json = "1.0"`
- `symphonia = { version = "0.5.4", features = ["flac", "wav"] }` (pure Rust audio decoding for backing tracks & test fixtures)
- `rubato = "0.15.0"` (high-quality sample rate converter for multi-device matching)

## 3. Audio Hardware Configuration & Defaults
- **Master Sample Rate:** `44100 Hz` (standard guitar audio capture)
- **Input Buffer Size:** `128` to `256 samples` ($2.9\text{--}5.8\text{ ms}$)
- **Output Buffer Size:** `256` to `512 samples` ($5.8\text{--}11.6\text{ ms}$)
- **Target Algorithmic DSP Latency:** $\le 30\text{ ms}$
- **Target Total Round-trip Audio Latency:** $\le 45\text{ ms}$ (hardware dependent)
- **Sub-200 Hz Decimation Factor:** $4\times$ (down to $11025\text{ Hz}$, 1024-sample window)
- **Pick Attack Transient Consensus Window:** $15\text{--}35\text{ ms}$ post-onset
- **Conditioning Low-Pass Filter:** 2nd-order Butterworth at $3500\text{ Hz}$

## 4. Concurrency & Thread Budget
- **Audio Thread:** Dedicated OS native thread with `THREAD_PRIORITY_TIME_CRITICAL` (Windows) / `SCHED_FIFO` (Linux) / `pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE)` (macOS). **Zero heap allocations, zero mutexes, zero disk I/O, zero network calls**.
- **DSP Worker Thread:** Dedicated worker reading audio blocks from Cubeb SPSC ring buffer, executing onset + pitch algorithms.
- **Main / Render Thread:** Godot 4.3 main game loop reading scored events and hardware sample counter from a lock-free queue, updating 3D highway transforms.
