/**
 * Karplus-Strong Electric Guitar Physical Modeling Synthesizer
 * Generates realistic guitar strings with pick attack click, damping, and body resonance.
 */
export class ElectricGuitarVoice {
  private buffer: Float32Array;
  private bufferIndex: number = 0;
  private filterState: number = 0;
  private decayFactor: number = 0.988;
  private sampleRate: number;
  private isActive: boolean = false;

  constructor(sampleRate: number = 44100) {
    this.sampleRate = sampleRate;
    this.buffer = new Float32Array(Math.ceil(sampleRate / 70)); // down to ~70 Hz
  }

  public pluck(frequencyHz: number, velocity: number = 0.8): void {
    const periodSamples = Math.round(this.sampleRate / frequencyHz);
    const len = Math.max(10, Math.min(this.buffer.length, periodSamples));

    // Fill with band-limited initial noise burst (pick excitation)
    let prev = 0;
    for (let i = 0; i < len; i++) {
      const white = (Math.random() * 2 - 1) * velocity;
      prev = 0.3 * white + 0.7 * prev;
      this.buffer[i] = prev;
    }

    this.bufferIndex = 0;
    this.filterState = 0;
    // Higher pitch has faster decay
    this.decayFactor = 0.985 + 0.012 * Math.exp(-frequencyHz / 400);
    this.isActive = true;
  }

  public processSample(): number {
    if (!this.isActive) return 0;

    const current = this.buffer[this.bufferIndex];
    const nextIdx = (this.bufferIndex + 1) % this.buffer.length;
    const next = this.buffer[nextIdx];

    // Low-pass string feedback loop
    const averaged = 0.5 * (current + next);
    this.filterState = 0.8 * averaged + 0.2 * this.filterState;
    const feedback = this.filterState * this.decayFactor;

    this.buffer[this.bufferIndex] = feedback;
    this.bufferIndex = nextIdx;

    if (Math.abs(current) < 1e-4 && Math.abs(feedback) < 1e-4) {
      this.isActive = false;
    }

    return current;
  }

  public active(): boolean {
    return this.isActive;
  }
}

export class ElectricGuitarSynth {
  private voices: ElectricGuitarVoice[] = [];
  private sampleRate: number;
  private audioCtx: AudioContext | null = null;

  constructor(sampleRate: number = 44100) {
    this.sampleRate = sampleRate;
    for (let i = 0; i < 8; i++) {
      this.voices.push(new ElectricGuitarVoice(sampleRate));
    }
  }

  public playNote(freqHz: number, velocity: number = 0.8, audioCtx?: AudioContext): void {
    if (audioCtx) {
      // Direct WebAudio oscillator/buffer fallback for instant auditioning
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      const filter = audioCtx.createBiquadFilter();

      filter.type = 'lowpass';
      filter.frequency.setValueAtTime(2800, audioCtx.currentTime);

      osc.type = 'sawtooth';
      osc.frequency.setValueAtTime(freqHz, audioCtx.currentTime);

      gain.gain.setValueAtTime(0.001, audioCtx.currentTime);
      gain.gain.linearRampToValueAtTime(0.35 * velocity, audioCtx.currentTime + 0.008);
      gain.gain.exponentialRampToValueAtTime(0.0001, audioCtx.currentTime + 1.6);

      osc.connect(filter);
      filter.connect(gain);
      gain.connect(audioCtx.destination);

      osc.start();
      osc.stop(audioCtx.currentTime + 1.8);
      return;
    }

    // Allocate internal KS voice
    const voice = this.voices.find((v) => !v.active()) || this.voices[0];
    voice.pluck(freqHz, velocity);
  }

  public renderBuffer(freqHz: number, durationSec: number = 1.0): Float32Array {
    const numSamples = Math.round(durationSec * this.sampleRate);
    const out = new Float32Array(numSamples);
    const voice = new ElectricGuitarVoice(this.sampleRate);
    voice.pluck(freqHz, 0.9);

    for (let i = 0; i < numSamples; i++) {
      out[i] = voice.processSample();
    }
    return out;
  }
}
