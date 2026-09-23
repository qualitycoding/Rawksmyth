export interface OnsetDetectionEvent {
  timestampMs: number;
  strength: number;
}

/**
 * Adaptive Spectral Flux & High-Frequency Content Onset Detector (Decision D-004)
 * Incorporates moving RMS average dynamic thresholding and a 40 ms refractory lockout.
 */
export class SpectralFluxOnsetDetector {
  private sampleRate: number;
  private movingAvgEnergy: number = 0.001;
  private thresholdMultiplier: number = 1.8;
  private refractorySamples: number;
  private samplesSinceLastOnset: number = 100000;
  private elapsedSamples: number = 0;

  constructor(sampleRate: number = 44100) {
    this.sampleRate = sampleRate;
    this.refractorySamples = Math.round(sampleRate * 0.040); // 40ms refractory gate
  }

  public process(samples: Float32Array): OnsetDetectionEvent | null {
    this.elapsedSamples += samples.length;
    this.samplesSinceLastOnset += samples.length;

    let energy = 0;
    let hfc = 0;

    for (let i = 0; i < samples.length; i++) {
      const s = samples[i];
      const absS = Math.abs(s);
      energy += absS * absS;
      if (i > 0) {
        const diff = Math.abs(s - samples[i - 1]);
        hfc += diff * diff;
      }
    }

    energy /= Math.max(1, samples.length);
    hfc /= Math.max(1, samples.length);

    const dynamicThreshold = Math.max(0.002, this.movingAvgEnergy * this.thresholdMultiplier);
    this.movingAvgEnergy = 0.92 * this.movingAvgEnergy + 0.08 * energy;

    const timestampMs = Math.round((this.elapsedSamples * 1000) / this.sampleRate);

    if (this.samplesSinceLastOnset >= this.refractorySamples) {
      if (energy > dynamicThreshold && hfc > dynamicThreshold * 0.5) {
        this.samplesSinceLastOnset = 0;
        return {
          timestampMs,
          strength: Math.min(5.0, energy / dynamicThreshold)
        };
      }
    }

    return null;
  }

  public reset(): void {
    this.movingAvgEnergy = 0.001;
    this.samplesSinceLastOnset = 100000;
    this.elapsedSamples = 0;
  }
}
