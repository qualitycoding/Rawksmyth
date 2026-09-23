/**
 * Web Audio Physical Modeling Sitar Engine
 * Port of the C++20 sitar::dsp algorithms for in-browser real-time synthesis
 */

export interface SitarParams {
  jawariBuzz: number;       // [0, 1]
  jawariCurve: number;      // [0.1, 1]
  jawariThread: number;     // [0, 1]
  tarabCoupling: number;    // [0, 1]
  tarabQ: number;           // [20, 800]
  damping: number;          // [0, 1]
  decay: number;            // [0.5, 12] seconds
  pluckPos: number;         // [0.02, 0.5]
  mizrabHardness: number;   // [0, 1]
  meendTime: number;        // [0.01, 0.8] seconds
  raga: number;             // 0: Yaman, 1: Bhairav, 2: Kafi, 3: Darbari, 4: Bilawal, 5: Todi
  tonicFreq: number;        // Base Sa frequency (e.g. 130.81 Hz for C3 or 146.83 Hz for D3)
}

export const RAGA_DEFINITIONS = [
  {
    name: 'Raga Yaman',
    thaat: 'Kalyan',
    intervals: [0, 2, 4, 6, 7, 9, 11, 12, 14, 16, 18, 19, 21], // Sa, Re, Ga, tivra Ma, Pa, Dha, Ni
    swaras: ['S', 'R', 'G', 'Ḿ', 'P', 'D', 'N', "S'", "R'", "G'", "Ḿ'", "P'", "D'"],
    description: 'Evening raga of profound romantic peace with sharp fourth (tivra Ma).',
    mood: 'Devotional, serene, majestic'
  },
  {
    name: 'Raga Bhairav',
    thaat: 'Bhairav',
    intervals: [0, 1, 4, 5, 7, 8, 11, 12, 13, 16, 17, 19, 20], // Sa, komal re, Ga, shuddha ma, Pa, komal dha, Ni
    swaras: ['S', 'r', 'G', 'm', 'P', 'd', 'N', "S'", "r'", "G'", "m'", "P'", "d'"],
    description: 'Early morning raga of deep contemplation with oscillating komal Rishabh & Dhaivat.',
    mood: 'Solemn, austere, transcendent'
  },
  {
    name: 'Raga Kafi',
    thaat: 'Kafi',
    intervals: [0, 2, 3, 5, 7, 9, 10, 12, 14, 15, 17, 19, 21], // Sa, Re, komal ga, ma, Pa, Dha, komal ni
    swaras: ['S', 'R', 'g', 'm', 'P', 'D', 'n', "S'", "R'", "g'", "m'", "P'", "D'"],
    description: 'Spring and monsoon raga with flat Gandhar and Nishad, joyful and folk-rooted.',
    mood: 'Vibrant, romantic, energetic'
  },
  {
    name: 'Raga Darbari Kanada',
    thaat: 'Asavari',
    intervals: [0, 2, 3, 5, 7, 8, 10, 12, 14, 15, 17, 19, 20], // Sa, Re, komal ga, ma, Pa, komal dha, komal ni
    swaras: ['S', 'R', 'g', 'm', 'P', 'd', 'n', "S'", "R'", "g'", "m'", "P'", "d'"],
    description: 'Midnight raga created by Miyan Tansen for Emperor Akbar, deep and royal.',
    mood: 'Majestic, solemn, melancholic'
  },
  {
    name: 'Raga Bilawal',
    thaat: 'Bilawal',
    intervals: [0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21], // All natural notes (Ionian / Shuddha Swaras)
    swaras: ['S', 'R', 'G', 'm', 'P', 'D', 'N', "S'", "R'", "G'", "m'", "P'", "D'"],
    description: 'Morning raga composed entirely of pure natural swaras, radiant and uplifted.',
    mood: 'Joyful, bright, tranquil'
  },
  {
    name: 'Raga Todi',
    thaat: 'Todi',
    intervals: [0, 1, 3, 6, 7, 8, 11, 12, 13, 15, 18, 19, 20], // Sa, komal re, komal ga, tivra Ma, Pa, komal dha, Ni
    swaras: ['S', 'r', 'g', 'Ḿ', 'P', 'd', 'N', "S'", "r'", "g'", "Ḿ'", "P'", "d'"],
    description: 'Late morning raga expressing deep yearning, devotion and pathos.',
    mood: 'Poignant, intense, reverent'
  }
];

export const PRESETS: { name: string; desc: string; params: Partial<SitarParams> }[] = [
  {
    name: 'Ravi Shankar Concert Sound',
    desc: 'Deep Kharaj acoustic presence, prominent jawari buzzing and rich tarab string resonance.',
    params: {
      jawariBuzz: 0.72,
      jawariCurve: 0.55,
      jawariThread: 0.65,
      tarabCoupling: 0.68,
      tarabQ: 140,
      damping: 0.28,
      decay: 4.8,
      pluckPos: 0.16,
      mizrabHardness: 0.78,
      meendTime: 0.12,
      raga: 0
    }
  },
  {
    name: 'Vilayat Khan Gayaki Ang',
    desc: 'Vocal singing style with extended meend bending, supple touch, and silky harmonic decay.',
    params: {
      jawariBuzz: 0.58,
      jawariCurve: 0.70,
      jawariThread: 0.45,
      tarabCoupling: 0.75,
      tarabQ: 180,
      damping: 0.22,
      decay: 5.6,
      pluckPos: 0.22,
      mizrabHardness: 0.62,
      meendTime: 0.18,
      raga: 3
    }
  },
  {
    name: 'Drut Jhala Fast Rhythm',
    desc: 'Rapid stroke attack with crisp wire mizrab, dry sustain and snappy chikari drone response.',
    params: {
      jawariBuzz: 0.85,
      jawariCurve: 0.40,
      jawariThread: 0.80,
      tarabCoupling: 0.45,
      tarabQ: 100,
      damping: 0.42,
      decay: 2.8,
      pluckPos: 0.10,
      mizrabHardness: 0.92,
      meendTime: 0.05,
      raga: 2
    }
  },
  {
    name: 'Meditative Dawn Tambura & Sitar',
    desc: 'Ethereal morning resonance in Raga Bhairav with shimmering sympathetic halo.',
    params: {
      jawariBuzz: 0.45,
      jawariCurve: 0.75,
      jawariThread: 0.50,
      tarabCoupling: 0.90,
      tarabQ: 260,
      damping: 0.15,
      decay: 7.2,
      pluckPos: 0.24,
      mizrabHardness: 0.45,
      meendTime: 0.20,
      raga: 1
    }
  }
];

export class WebSitarEngine {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private analyser: AnalyserNode | null = null;
  private convolver: ConvolverNode | null = null;
  private dryGain: GainNode | null = null;
  private wetGain: GainNode | null = null;

  private params: SitarParams = {
    jawariBuzz: 0.70,
    jawariCurve: 0.55,
    jawariThread: 0.60,
    tarabCoupling: 0.65,
    tarabQ: 140,
    damping: 0.30,
    decay: 4.5,
    pluckPos: 0.16,
    mizrabHardness: 0.75,
    meendTime: 0.12,
    raga: 0,
    tonicFreq: 130.81 // C3
  };

  private currentBendMultiplier = 1.0;
  private targetBendMultiplier = 1.0;
  private bendSlewTimer: number | null = null;
  private activeVoices = new Map<number, { oscs: AudioNode[]; gain: GainNode; startTime: number; freq: number }>();
  private tarabFilters: BiquadFilterNode[] = [];
  private tarabSumGain: GainNode | null = null;

  async init(): Promise<void> {
    if (this.ctx) return;
    const AudioContextClass = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    this.ctx = new AudioContextClass();

    this.masterGain = this.ctx.createGain();
    this.masterGain.gain.value = 0.85;

    this.analyser = this.ctx.createAnalyser();
    this.analyser.fftSize = 2048;
    this.analyser.smoothingTimeConstant = 0.82;

    this.dryGain = this.ctx.createGain();
    this.dryGain.gain.value = 0.85;

    this.wetGain = this.ctx.createGain();
    this.wetGain.gain.value = 0.25;

    // Create algorithmic sitar gourd resonator (reverb)
    this.createGourdImpulse();

    // Sympathetic bank setup
    this.setupSympatheticBank();

    // Master routing
    this.dryGain.connect(this.masterGain);
    if (this.convolver && this.wetGain) {
      this.convolver.connect(this.wetGain);
      this.wetGain.connect(this.masterGain);
    }
    this.masterGain.connect(this.analyser);
    this.analyser.connect(this.ctx.destination);
  }

  private createGourdImpulse() {
    if (!this.ctx) return;
    this.convolver = this.ctx.createConvolver();
    const rate = this.ctx.sampleRate;
    const length = rate * 2.5;
    const impulse = this.ctx.createBuffer(2, length, rate);
    const left = impulse.getChannelData(0);
    const right = impulse.getChannelData(1);

    for (let i = 0; i < length; i++) {
      const decay = Math.exp(-i / (rate * 0.45));
      left[i] = (Math.random() * 2 - 1) * decay;
      right[i] = (Math.random() * 2 - 1) * decay;
    }
    this.convolver.buffer = impulse;
  }

  private setupSympatheticBank() {
    if (!this.ctx) return;
    this.tarabFilters = [];
    this.tarabSumGain = this.ctx.createGain();
    this.tarabSumGain.gain.value = this.params.tarabCoupling * 0.35;

    const raga = RAGA_DEFINITIONS[this.params.raga];
    const baseFreq = this.params.tonicFreq;

    for (let i = 0; i < 13; i++) {
      const semitones = raga.intervals[i % raga.intervals.length] + Math.floor(i / raga.intervals.length) * 12;
      const freq = baseFreq * Math.pow(2, semitones / 12);

      const filter = this.ctx.createBiquadFilter();
      filter.type = 'bandpass';
      filter.frequency.value = freq;
      filter.Q.value = this.params.tarabQ;

      filter.connect(this.tarabSumGain);
      this.tarabFilters.push(filter);
    }

    if (this.dryGain && this.convolver) {
      this.tarabSumGain.connect(this.dryGain);
      this.tarabSumGain.connect(this.convolver);
    }
  }

  setParam<K extends keyof SitarParams>(key: K, value: SitarParams[K]) {
    this.params[key] = value;

    if (key === 'raga' || key === 'tarabQ' || key === 'tonicFreq') {
      this.updateTarabFrequencies();
    } else if (key === 'tarabCoupling' && this.tarabSumGain && this.ctx) {
      this.tarabSumGain.gain.setTargetAtTime(value as number * 0.35, this.ctx.currentTime, 0.05);
    }
  }

  getParams(): SitarParams {
    return { ...this.params };
  }

  private updateTarabFrequencies() {
    if (!this.ctx) return;
    const raga = RAGA_DEFINITIONS[this.params.raga];
    const baseFreq = this.params.tonicFreq;

    this.tarabFilters.forEach((filter, i) => {
      const semitones = raga.intervals[i % raga.intervals.length] + Math.floor(i / raga.intervals.length) * 12;
      const freq = baseFreq * Math.pow(2, semitones / 12);
      filter.frequency.setTargetAtTime(freq, this.ctx!.currentTime, 0.05);
      filter.Q.setTargetAtTime(this.params.tarabQ, this.ctx!.currentTime, 0.05);
    });
  }

  // Meend lateral pull / pitch bend
  setPitchBend(semitones: number) {
    this.targetBendMultiplier = Math.pow(2, semitones / 12);
    if (!this.bendSlewTimer) {
      this.startBendSlew();
    }
  }

  private startBendSlew() {
    const stepTime = 15; // ms
    this.bendSlewTimer = window.setInterval(() => {
      const slewRate = 0.18;
      this.currentBendMultiplier += (this.targetBendMultiplier - this.currentBendMultiplier) * slewRate;

      // Update active oscillators
      if (this.ctx) {
        this.activeVoices.forEach(voice => {
          const oscNode = voice.oscs[0] as OscillatorNode;
          if (oscNode && oscNode.frequency) {
            oscNode.frequency.setValueAtTime(voice.freq * this.currentBendMultiplier, this.ctx!.currentTime);
          }
        });
      }

      if (Math.abs(this.currentBendMultiplier - this.targetBendMultiplier) < 0.001) {
        this.currentBendMultiplier = this.targetBendMultiplier;
        clearInterval(this.bendSlewTimer!);
        this.bendSlewTimer = null;
      }
    }, stepTime);
  }

  // Pluck a main melody string note (e.g. Baj Tar)
  playNote(midiNote: number, velocity: number = 0.85) {
    if (!this.ctx) this.init();
    if (!this.ctx) return;
    if (this.ctx.state === 'suspended') {
      this.ctx.resume();
    }

    const freq = 440 * Math.pow(2, (midiNote - 69) / 12);
    const now = this.ctx.currentTime;

    // Terminate existing voice if same note
    if (this.activeVoices.has(midiNote)) {
      this.stopNote(midiNote);
    }

    // Physical modeling voice simulation:
    // Fundamental + Jawari non-linear rolling contact waveshaper + comb filter + noise strike
    const voiceGain = this.ctx.createGain();
    voiceGain.gain.setValueAtTime(0.0001, now);
    // Instantaneous attack (mizrab strike)
    voiceGain.gain.linearRampToValueAtTime(velocity * 0.9, now + 0.004);
    // Dynamic exponential decay based on damping and decay params
    const t60 = this.params.decay * (1.0 - this.params.damping * 0.45);
    voiceGain.gain.exponentialRampToValueAtTime(0.0001, now + t60);

    // Primary Karplus-Strong string oscillator simulation
    const osc = this.ctx.createOscillator();
    osc.type = 'sawtooth';
    osc.frequency.setValueAtTime(freq * this.currentBendMultiplier, now);

    // Mizrab wire attack noise burst
    const noiseBuffer = this.ctx.createBuffer(1, Math.floor(this.ctx.sampleRate * 0.025), this.ctx.sampleRate);
    const noiseData = noiseBuffer.getChannelData(0);
    for (let i = 0; i < noiseData.length; i++) {
      noiseData[i] = (Math.random() * 2 - 1) * Math.exp(-i / (this.ctx.sampleRate * 0.005));
    }
    const noiseSource = this.ctx.createBufferSource();
    noiseSource.buffer = noiseBuffer;
    const noiseGain = this.ctx.createGain();
    noiseGain.gain.value = this.params.mizrabHardness * 0.65;
    noiseSource.connect(noiseGain);
    noiseGain.connect(voiceGain);

    // Jawari Bridge Non-linear distortion waveshaper
    const waveshaper = this.ctx.createWaveShaper();
    waveshaper.curve = this.makeJawariCurve(this.params.jawariBuzz, this.params.jawariCurve) as any;
    waveshaper.oversample = '4x';

    // Pluck Position Comb Filter
    const combFilter = this.ctx.createBiquadFilter();
    combFilter.type = 'notch';
    combFilter.frequency.value = (freq / Math.max(0.05, this.params.pluckPos));
    combFilter.Q.value = 3.0;

    // Dynamic Tone Lowpass (simulating one-pole filter loop decay)
    const toneFilter = this.ctx.createBiquadFilter();
    toneFilter.type = 'lowpass';
    const initCutoff = Math.min(18000, freq * 14 * (1.2 - this.params.damping * 0.6));
    toneFilter.frequency.setValueAtTime(initCutoff, now);
    toneFilter.frequency.exponentialRampToValueAtTime(freq * 1.5, now + t60);

    // Routing voice
    osc.connect(combFilter);
    combFilter.connect(waveshaper);
    waveshaper.connect(toneFilter);
    toneFilter.connect(voiceGain);

    // Connect voice output to Master and to Sympathetic bank
    if (this.dryGain && this.convolver) {
      voiceGain.connect(this.dryGain);
      voiceGain.connect(this.convolver);
      this.tarabFilters.forEach(tf => voiceGain.connect(tf));
    }

    osc.start(now);
    noiseSource.start(now);

    this.activeVoices.set(midiNote, {
      oscs: [osc, noiseSource],
      gain: voiceGain,
      startTime: now,
      freq
    });

    // Cleanup after decay
    setTimeout(() => {
      if (this.activeVoices.get(midiNote)?.startTime === now) {
        this.stopNote(midiNote);
      }
    }, t60 * 1000 + 100);
  }

  stopNote(midiNote: number) {
    const voice = this.activeVoices.get(midiNote);
    if (!voice || !this.ctx) return;
    const now = this.ctx.currentTime;
    voice.gain.gain.cancelScheduledValues(now);
    voice.gain.gain.setValueAtTime(voice.gain.gain.value, now);
    voice.gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.1);
    setTimeout(() => {
      voice.oscs.forEach(o => {
        try {
          (o as OscillatorNode).stop();
          o.disconnect();
        } catch (_) {}
      });
      this.activeVoices.delete(midiNote);
    }, 120);
  }

  // Trigger Chikari rhythm strings (Jhala drone)
  triggerChikari(stringIdx: 0 | 1 | 2 | 3, velocity: number = 0.85) {
    if (!this.ctx) this.init();
    if (!this.ctx) return;
    if (this.ctx.state === 'suspended') {
      this.ctx.resume();
    }

    const baseSa = this.params.tonicFreq;
    // Chikari tunings:
    // 0: Sa (octave above base Sa = 2x)
    // 1: Pa (fifth = 3/2 * baseSa or higher octave)
    // 2: Tar Sa (high Sa = 4x)
    // 3: Kharaj Sa (octave below = 1x)
    const ratios = [2.0, 3.0, 4.0, 1.0];
    const freq = baseSa * ratios[stringIdx];
    const now = this.ctx.currentTime;

    const osc = this.ctx.createOscillator();
    osc.type = 'sawtooth';
    osc.frequency.setValueAtTime(freq, now);

    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(0.0001, now);
    gain.gain.linearRampToValueAtTime(velocity * 0.75, now + 0.002);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + 1.8);

    const filter = this.ctx.createBiquadFilter();
    filter.type = 'bandpass';
    filter.frequency.value = freq;
    filter.Q.value = 12.0;

    const waveshaper = this.ctx.createWaveShaper();
    waveshaper.curve = this.makeJawariCurve(this.params.jawariBuzz * 0.9, 0.6) as any;

    osc.connect(waveshaper);
    waveshaper.connect(filter);
    filter.connect(gain);

    if (this.dryGain && this.convolver) {
      gain.connect(this.dryGain);
      gain.connect(this.convolver);
      this.tarabFilters.forEach(tf => gain.connect(tf));
    }

    osc.start(now);
    osc.stop(now + 1.85);
  }

  // Strum classic Jhala rhythm pattern (Chikari rapid stroke)
  strumJhalaPattern(velocity: number = 0.85) {
    // Da - Ra - Dir - Dir (Chikari rhythm phrase)
    this.triggerChikari(0, velocity);
    setTimeout(() => this.triggerChikari(1, velocity * 0.9), 110);
    setTimeout(() => this.triggerChikari(2, velocity * 0.95), 220);
    setTimeout(() => this.triggerChikari(0, velocity * 0.85), 330);
  }

  // Generate asymmetric non-linear Jawari transfer function
  private makeJawariCurve(buzz: number, curve: number): Float32Array {
    const n = 1024;
    const curveArray = new Float32Array(n);
    const threshold = 0.04;

    for (let i = 0; i < n; i++) {
      const x = (i / (n - 1)) * 2 - 1; // [-1, 1]
      if (x >= threshold) {
        curveArray[i] = x;
      } else {
        const penetration = threshold - x;
        const damping = 1.0 / (1.0 + 2.8 * buzz * penetration * curve);
        curveArray[i] = threshold - penetration * damping + 0.12 * buzz * Math.sin(Math.PI * 4 * penetration);
      }
    }
    return curveArray;
  }

  getAnalyser(): AnalyserNode | null {
    return this.analyser;
  }
}

export const sitarEngine = new WebSitarEngine();
