import { ButterworthLowPass } from './conditioningFilter';
import { Decimator4x } from './decimator';

export interface PitchEstimate {
  frequencyHz: number;
  confidence: number;
  midiNote: number;
  centsDeviation: number;
  timestampMs: number;
  isVoiced: boolean;
}

/**
 * McLeod Pitch Method (MPM) & Multi-Rate Hybrid Estimator (Decisions D-002, D-008).
 * Accurately tracks E2 (82.41 Hz) up to E5 (659.26 Hz) with <= 10 cents accuracy.
 */
export class McLeodPitchDetector {
  private sampleRate: number;
  private conditioning: ButterworthLowPass;
  private decimator: Decimator4x;
  private energyThreshold: number = 0.006;
  private clarityThreshold: number = 0.65;
  private frameCount: number = 0;
  private hopSize: number = 256;

  constructor(sampleRate: number = 44100, hopSize: number = 256) {
    this.sampleRate = sampleRate;
    this.hopSize = hopSize;
    this.conditioning = new ButterworthLowPass(3500, sampleRate);
    this.decimator = new Decimator4x(sampleRate);
  }

  public static hzToMidi(hz: number): { midiNote: number; cents: number } {
    if (hz <= 0 || !isFinite(hz)) {
      return { midiNote: 0, cents: 0 };
    }
    const midiFloat = 69 + 12 * Math.log2(hz / 440);
    const roundedMidi = Math.round(midiFloat);
    const cents = (midiFloat - roundedMidi) * 100;
    return {
      midiNote: Math.max(0, Math.min(127, roundedMidi)),
      cents
    };
  }

  public static midiToNoteName(midi: number): string {
    const names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
    const octave = Math.floor(midi / 12) - 1;
    const note = names[midi % 12];
    return `${note}${octave}`;
  }

  /**
   * Compute Normalized Square Difference Function (NSDF)
   */
  private computeNsdf(buffer: Float32Array, maxTau: number): Float32Array {
    const n = buffer.length;
    const nsdf = new Float32Array(maxTau);
    if (n < maxTau) return nsdf;

    for (let tau = 0; tau < maxTau; tau++) {
      let acf = 0;
      let m0 = 0;
      let mTau = 0;

      for (let j = 0; j < n - tau; j++) {
        const s0 = buffer[j];
        const st = buffer[j + tau];
        acf += s0 * st;
        m0 += s0 * s0;
        mTau += st * st;
      }

      const m = m0 + mTau;
      nsdf[tau] = m > 1e-8 ? (2 * acf) / m : 0;
    }

    return nsdf;
  }

  /**
   * Parabolic interpolation around peak index
   */
  private parabolicInterpolation(nsdf: Float32Array, peakIdx: number): { tau: number; clarity: number } {
    if (peakIdx <= 0 || peakIdx >= nsdf.length - 1) {
      return { tau: peakIdx, clarity: nsdf[peakIdx] };
    }

    const alpha = nsdf[peakIdx - 1];
    const beta = nsdf[peakIdx];
    const gamma = nsdf[peakIdx + 1];

    const denom = 2 * (alpha - 2 * beta + gamma);
    if (Math.abs(denom) < 1e-6) {
      return { tau: peakIdx, clarity: beta };
    }

    const delta = (alpha - gamma) / denom;
    const turningPoint = peakIdx + delta;
    const peakValue = beta - 0.25 * (alpha - gamma) * delta;

    return { tau: turningPoint, clarity: peakValue };
  }

  public detectRaw(buffer: Float32Array, sr: number, minFreq: number, maxFreq: number): { freq: number; clarity: number } | null {
    // Check RMS energy
    let sumSq = 0;
    for (let i = 0; i < buffer.length; i++) {
      sumSq += buffer[i] * buffer[i];
    }
    const rms = Math.sqrt(sumSq / buffer.length);
    if (rms < this.energyThreshold) {
      return null;
    }

    const minTau = Math.floor(sr / maxFreq);
    const maxTau = Math.ceil(sr / minFreq);
    if (maxTau >= buffer.length) return null;

    const nsdf = this.computeNsdf(buffer, maxTau + 2);

    let zeroCrossed = false;
    let bestTau = 0;
    let maxClarity = 0;

    for (let i = Math.max(1, minTau); i < maxTau; i++) {
      if (nsdf[i] < 0) {
        zeroCrossed = true;
      }
      if (zeroCrossed && nsdf[i] > 0 && nsdf[i] >= nsdf[i - 1] && nsdf[i] >= nsdf[i + 1]) {
        const { tau, clarity } = this.parabolicInterpolation(nsdf, i);
        if (clarity > this.clarityThreshold) {
          return { freq: sr / tau, clarity };
        }
        if (clarity > maxClarity) {
          maxClarity = clarity;
          bestTau = tau;
        }
      }
    }

    if (maxClarity >= this.clarityThreshold && bestTau > 0) {
      return { freq: sr / bestTau, clarity: maxClarity };
    }

    return null;
  }

  public process(samples: Float32Array): PitchEstimate | null {
    this.frameCount++;
    const timestampMs = Math.round((this.frameCount * this.hopSize * 1000) / this.sampleRate);

    // 1. Condition signal through 3.5 kHz low-pass filter
    const conditioned = new Float32Array(samples.length);
    this.conditioning.processBlock(samples, conditioned);

    // 2. Mid/High register pass (140 Hz to 750 Hz)
    const midHighResult = this.detectRaw(conditioned, this.sampleRate, 140, 750);
    if (midHighResult && midHighResult.clarity >= this.clarityThreshold) {
      const { midiNote, cents } = McLeodPitchDetector.hzToMidi(midHighResult.freq);
      return {
        frequencyHz: midHighResult.freq,
        confidence: midHighResult.clarity,
        midiNote,
        centsDeviation: cents,
        timestampMs,
        isVoiced: true
      };
    }

    // 3. Multi-rate 4x Decimated pass for low register (E2 82.41 Hz to 180 Hz)
    const decimated = this.decimator.process(conditioned);
    const decimatedSr = this.sampleRate / 4;
    const lowResult = this.detectRaw(decimated, decimatedSr, 75, 180);

    if (lowResult && lowResult.clarity >= this.clarityThreshold) {
      const { midiNote, cents } = McLeodPitchDetector.hzToMidi(lowResult.freq);
      return {
        frequencyHz: lowResult.freq,
        confidence: lowResult.clarity,
        midiNote,
        centsDeviation: cents,
        timestampMs,
        isVoiced: true
      };
    }

    return null;
  }

  public reset(): void {
    this.conditioning.reset();
    this.decimator.reset();
    this.frameCount = 0;
  }
}
