import { ButterworthLowPass } from './conditioningFilter';

/**
 * 4x Multi-Rate Decimation Filter (Decision D-002)
 * Downsamples from 44.1 kHz to 11.025 kHz with anti-aliasing.
 * Provides ~7.6 periods of E2 (82.41 Hz) in a 1024-sample window with zero latency penalty.
 */
export class Decimator4x {
  private antiAlias: ButterworthLowPass;

  constructor(sampleRate: number = 44100) {
    // Anti-alias low-pass filter set at 2500 Hz (< 11025 / 4 = 2756 Hz Nyquist)
    this.antiAlias = new ButterworthLowPass(2500, sampleRate);
  }

  public process(input: Float32Array): Float32Array {
    const outLen = Math.floor(input.length / 4);
    const output = new Float32Array(outLen);
    let outIdx = 0;

    for (let i = 0; i < input.length; i++) {
      const filtered = this.antiAlias.processSample(input[i]);
      if (i % 4 === 0 && outIdx < outLen) {
        output[outIdx++] = filtered;
      }
    }

    return output;
  }

  public reset(): void {
    this.antiAlias.reset();
  }
}
