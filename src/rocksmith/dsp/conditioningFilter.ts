/**
 * 2nd-Order Butterworth Low-Pass Filter (Decision D-008)
 * Rolls off electromagnetic pickup hum, pick scrapes, and fret rattles above 3.5 kHz.
 */
export class ButterworthLowPass {
  private b0: number = 0;
  private b1: number = 0;
  private b2: number = 0;
  private a1: number = 0;
  private a2: number = 0;
  private x1: number = 0;
  private x2: number = 0;
  private y1: number = 0;
  private y2: number = 0;

  constructor(cutoffHz: number = 3500, sampleRate: number = 44100) {
    this.configure(cutoffHz, sampleRate);
  }

  public configure(cutoffHz: number, sampleRate: number): void {
    const w0 = (2 * Math.PI * cutoffHz) / sampleRate;
    const cosW0 = Math.cos(w0);
    const sinW0 = Math.sin(w0);
    const alpha = sinW0 / (2 * 0.70710678); // Q = 1 / sqrt(2)

    const a0 = 1 + alpha;
    this.b0 = (1 - cosW0) / 2 / a0;
    this.b1 = (1 - cosW0) / a0;
    this.b2 = (1 - cosW0) / 2 / a0;
    this.a1 = (-2 * cosW0) / a0;
    this.a2 = (1 - alpha) / a0;

    this.reset();
  }

  public processSample(x: number): number {
    const y =
      this.b0 * x +
      this.b1 * this.x1 +
      this.b2 * this.x2 -
      this.a1 * this.y1 -
      this.a2 * this.y2;

    this.x2 = this.x1;
    this.x1 = x;
    this.y2 = this.y1;
    this.y1 = y;

    return y;
  }

  public processBlock(input: Float32Array, output: Float32Array): void {
    for (let i = 0; i < input.length; i++) {
      output[i] = this.processSample(input[i]);
    }
  }

  public reset(): void {
    this.x1 = 0;
    this.x2 = 0;
    this.y1 = 0;
    this.y2 = 0;
  }
}
