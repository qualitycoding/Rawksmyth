import { McLeodPitchDetector, PitchEstimate } from '../dsp/mcLeodPitch';
import { SpectralFluxOnsetDetector, OnsetDetectionEvent } from '../dsp/spectralOnset';
import { TemporalNoteTracker, ResolvedNoteEvent } from '../dsp/temporalTracker';
import { ElectricGuitarSynth } from '../dsp/guitarSynthesizer';
import { ButterworthLowPass } from '../dsp/conditioningFilter';

export interface TunerState {
  detectedFreq: number;
  closestNote: string;
  targetFreq: number;
  cents: number;
  inTune: boolean;
  confidence: number;
}

export class RocksmithAudioEngine {
  private audioCtx: AudioContext | null = null;
  private mediaStream: MediaStream | null = null;
  private sourceNode: MediaStreamAudioSourceNode | null = null;
  private processorNode: ScriptProcessorNode | null = null;
  private analyserNode: AnalyserNode | null = null;

  public pitchDetector: McLeodPitchDetector;
  public onsetDetector: SpectralFluxOnsetDetector;
  public temporalTracker: TemporalNoteTracker;
  public guitarSynth: ElectricGuitarSynth;
  private conditioningFilter: ButterworthLowPass;

  public isRunning: boolean = false;
  public isMicrophoneActive: boolean = false;
  public inputGain: number = 1.0;
  public audioOffsetMs: number = 0;
  public visualOffsetMs: number = 0;

  // Real-time visual diagnostic buffers
  public rawTimeData: Float32Array = new Float32Array(512);
  public conditionedTimeData: Float32Array = new Float32Array(512);
  public frequencyData: Uint8Array = new Uint8Array(256);

  public latestPitch: PitchEstimate | null = null;
  public latestOnset: OnsetDetectionEvent | null = null;
  public tunerState: TunerState = {
    detectedFreq: 0,
    closestNote: '-',
    targetFreq: 0,
    cents: 0,
    inTune: false,
    confidence: 0,
  };

  private noteCallbacks: ((event: ResolvedNoteEvent) => void)[] = [];

  constructor() {
    this.pitchDetector = new McLeodPitchDetector(44100, 256);
    this.onsetDetector = new SpectralFluxOnsetDetector(44100);
    this.temporalTracker = new TemporalNoteTracker();
    this.guitarSynth = new ElectricGuitarSynth(44100);
    this.conditioningFilter = new ButterworthLowPass(3500, 44100);
  }

  public onResolvedNote(callback: (event: ResolvedNoteEvent) => void): void {
    this.noteCallbacks.push(callback);
  }

  public async startAudioContext(): Promise<void> {
    if (!this.audioCtx) {
      const AudioCtxClass = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      this.audioCtx = new AudioCtxClass({
        latencyHint: 'interactive',
        sampleRate: 44100,
      });
    }
    if (this.audioCtx.state === 'suspended') {
      await this.audioCtx.resume();
    }
    this.isRunning = true;
  }

  public async enableMicrophone(): Promise<boolean> {
    await this.startAudioContext();
    if (!this.audioCtx) return false;

    try {
      this.mediaStream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: false,
          noiseSuppression: false,
          autoGainControl: false,
          channelCount: 1,
        },
      });

      this.sourceNode = this.audioCtx.createMediaStreamSource(this.mediaStream);
      this.analyserNode = this.audioCtx.createAnalyser();
      this.analyserNode.fftSize = 512;
      this.frequencyData = new Uint8Array(this.analyserNode.frequencyBinCount);

      // ScriptProcessor buffer 512 samples = ~11.6ms at 44.1kHz
      this.processorNode = this.audioCtx.createScriptProcessor(512, 1, 1);
      this.processorNode.onaudioprocess = (e) => {
        const inputData = e.inputBuffer.getChannelData(0);
        this.processRealtimeAudio(inputData);
      };

      this.sourceNode.connect(this.analyserNode);
      this.sourceNode.connect(this.processorNode);
      // Dummy destination connection to keep ScriptProcessor running
      const dummyGain = this.audioCtx.createGain();
      dummyGain.gain.value = 0;
      this.processorNode.connect(dummyGain);
      dummyGain.connect(this.audioCtx.destination);

      this.isMicrophoneActive = true;
      return true;
    } catch (err) {
      console.warn('Microphone access not available or denied, falling back to simulated input:', err);
      this.isMicrophoneActive = false;
      return false;
    }
  }

  public disableMicrophone(): void {
    if (this.mediaStream) {
      this.mediaStream.getTracks().forEach((track) => track.stop());
      this.mediaStream = null;
    }
    if (this.processorNode) {
      this.processorNode.disconnect();
      this.processorNode = null;
    }
    if (this.sourceNode) {
      this.sourceNode.disconnect();
      this.sourceNode = null;
    }
    this.isMicrophoneActive = false;
  }

  public processRealtimeAudio(inputSamples: Float32Array): void {
    // 1. Copy raw input samples and apply input gain
    for (let i = 0; i < inputSamples.length; i++) {
      this.rawTimeData[i] = inputSamples[i] * this.inputGain;
    }

    // 2. Condition signal (3.5 kHz low-pass)
    this.conditioningFilter.processBlock(this.rawTimeData, this.conditionedTimeData);

    // 3. Extract spectral magnitudes if analyser is active
    if (this.analyserNode) {
      this.analyserNode.getByteFrequencyData(this.frequencyData as any);
    }

    // 4. Onset detection
    const onset = this.onsetDetector.process(this.conditionedTimeData);
    if (onset) {
      this.latestOnset = onset;
      this.temporalTracker.recordOnset(onset);
    }

    // 5. Pitch detection
    const pitch = this.pitchDetector.process(this.conditionedTimeData);
    if (pitch && pitch.isVoiced) {
      this.latestPitch = pitch;
      this.temporalTracker.recordPitch(pitch);
      this.updateTunerState(pitch);
    }

    // 6. Poll temporal reconciler
    const nowMs = this.audioCtx ? Math.round(this.audioCtx.currentTime * 1000) : performance.now();
    const resolvedNotes = this.temporalTracker.pollResolvedNotes(nowMs);
    for (const note of resolvedNotes) {
      for (const cb of this.noteCallbacks) {
        cb(note);
      }
    }
  }

  private updateTunerState(pitch: PitchEstimate): void {
    const { midiNote, cents } = McLeodPitchDetector.hzToMidi(pitch.frequencyHz);
    const targetFreq = 440 * Math.pow(2, (midiNote - 69) / 12);
    const noteName = McLeodPitchDetector.midiToNoteName(midiNote);

    this.tunerState = {
      detectedFreq: Math.round(pitch.frequencyHz * 10) / 10,
      closestNote: noteName,
      targetFreq: Math.round(targetFreq * 10) / 10,
      cents: Math.round(cents * 10) / 10,
      inTune: Math.abs(cents) <= 8,
      confidence: Math.round(pitch.confidence * 100),
    };
  }

  public playSyntheticPluck(freqHz: number, velocity: number = 0.8): void {
    if (this.audioCtx) {
      this.guitarSynth.playNote(freqHz, velocity, this.audioCtx);
    }
  }

  public playMetronomeClick(accent: boolean = false): void {
    if (!this.audioCtx) return;
    const osc = this.audioCtx.createOscillator();
    const gain = this.audioCtx.createGain();

    osc.frequency.value = accent ? 1200 : 800;
    osc.type = 'sine';

    gain.gain.setValueAtTime(0.3, this.audioCtx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, this.audioCtx.currentTime + 0.05);

    osc.connect(gain);
    gain.connect(this.audioCtx.destination);

    osc.start();
    osc.stop(this.audioCtx.currentTime + 0.06);
  }

  public getAudioTimeMs(): number {
    if (!this.audioCtx) return 0;
    return Math.max(0, Math.round(this.audioCtx.currentTime * 1000) + this.audioOffsetMs);
  }

  public getVisualTimeMs(): number {
    return Math.max(0, this.getAudioTimeMs() + this.visualOffsetMs);
  }
}
