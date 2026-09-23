# Research Questions & Inquiries (research/QUESTIONS.md)

1. **Audio I/O & Latency Isolation:**
   - What is the minimal achievable buffer size with `cubeb` on Windows 11 with WASAPI in Exclusive Mode vs Shared Mode?
   - How can we guarantee that the high-priority OS audio thread is never blocked by Godot's render thread or GC/allocation cycles?
   - *Status: Resolved via Decision D-001 (duplex stream) and D-003 (SPSC ring buffer).*

2. **Pitch Detection & Mathematical Limits:**
   - How do we track low $E_2$ ($82.41\text{ Hz}$) accurately without incurring large window delays ($>50\text{ ms}$)?
   - *Status: Resolved via Decision D-002 (Multi-rate $4\times$ decimation down to $11.025\text{ kHz}$).*

3. **Guitar Pick Physics vs Onset Timing:**
   - How does the system prevent the pick-strike transient from confusing the pitch detection algorithm?
   - *Status: Resolved via Decision D-006 (`TemporalNoteTracker` with 15–35 ms consensus window).*

4. **Engine & Rendering Decoupling:**
   - How does Godot receive game events and note updates without interfering with audio timing?
   - *Status: Resolved via Decision D-003 (GDExtension slave renderer reading lock-free queues).*
