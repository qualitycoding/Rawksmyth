//! Minimal WAV (RIFF/WAVE) support: PCM 8/16/24/32-bit and IEEE float32 read;
//! PCM16 write. Reading never panics: every malformed input is an `Err`.

use std::fmt;

/// Decoded audio: interleaved samples in [-1, 1].
#[derive(Debug, Clone, PartialEq)]
pub struct Wav {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl Wav {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }

    /// Average of all channels, one sample per frame.
    pub fn to_mono(&self) -> Vec<f32> {
        let c = self.channels.max(1) as usize;
        self.samples.chunks_exact(c).map(|f| f.iter().sum::<f32>() / c as f32).collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WavError {
    NotRiffWave,
    Truncated,
    MissingChunk(&'static str),
    Unsupported(String),
    TooLarge,
}

impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WavError::NotRiffWave => write!(f, "not a RIFF/WAVE file"),
            WavError::Truncated => write!(f, "truncated WAV data"),
            WavError::MissingChunk(c) => write!(f, "missing '{c}' chunk"),
            WavError::Unsupported(s) => write!(f, "unsupported WAV format: {s}"),
            WavError::TooLarge => write!(f, "WAV file exceeds the size limit"),
        }
    }
}

impl std::error::Error for WavError {}

/// Refuse absurd files (a 10-minute stereo 44.1 kHz float file is ~210 MB).
pub const MAX_WAV_BYTES: usize = 256 * 1024 * 1024;

fn u16_at(b: &[u8], o: usize) -> Result<u16, WavError> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or(WavError::Truncated)
}

fn u32_at(b: &[u8], o: usize) -> Result<u32, WavError> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or(WavError::Truncated)
}

pub fn parse(bytes: &[u8]) -> Result<Wav, WavError> {
    if bytes.len() > MAX_WAV_BYTES {
        return Err(WavError::TooLarge);
    }
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::NotRiffWave);
    }
    let mut pos = 12usize;
    let mut fmt: Option<(u16, u16, u32, u16)> = None; // (format tag, channels, rate, bits)
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32_at(bytes, pos + 4)? as usize;
        let body = pos + 8;
        let end = body.checked_add(size).ok_or(WavError::Truncated)?;
        match id {
            b"fmt " => {
                if size < 16 || end > bytes.len() {
                    return Err(WavError::Truncated);
                }
                let mut tag = u16_at(bytes, body)?;
                let channels = u16_at(bytes, body + 2)?;
                let rate = u32_at(bytes, body + 4)?;
                let bits = u16_at(bytes, body + 14)?;
                if tag == 0xFFFE && size >= 26 {
                    // WAVE_FORMAT_EXTENSIBLE: the real tag is the first two
                    // bytes of the sub-format GUID.
                    tag = u16_at(bytes, body + 24)?;
                }
                fmt = Some((tag, channels, rate, bits));
            }
            b"data" => {
                // Tolerate a data chunk whose declared size overruns the file
                // (streamed writers): take what is there.
                data = Some(&bytes[body..end.min(bytes.len())]);
            }
            _ => {}
        }
        // Chunks are word-aligned.
        pos = end.saturating_add(size & 1);
        if end > bytes.len() {
            break;
        }
    }
    let (tag, channels, rate, bits) = fmt.ok_or(WavError::MissingChunk("fmt "))?;
    let data = data.ok_or(WavError::MissingChunk("data"))?;
    if channels == 0 || rate == 0 {
        return Err(WavError::Unsupported("zero channels or sample rate".into()));
    }
    let bytes_per = (bits / 8) as usize;
    if bytes_per == 0 || bits % 8 != 0 {
        return Err(WavError::Unsupported(format!("{bits} bits per sample")));
    }
    let frame_bytes = bytes_per * channels as usize;
    let usable = data.len() / frame_bytes * frame_bytes;
    let data = &data[..usable];
    let samples: Vec<f32> = match (tag, bits) {
        (1, 8) => data.iter().map(|&b| (b as f32 - 128.0) / 128.0).collect(),
        (1, 16) => data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect(),
        (1, 24) => data
            .chunks_exact(3)
            .map(|c| (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0)
            .collect(),
        (1, 32) => data
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0)
            .collect(),
        (3, 32) => data.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect(),
        _ => return Err(WavError::Unsupported(format!("format tag {tag}, {bits} bits"))),
    };
    Ok(Wav { sample_rate: rate, channels, samples })
}

/// Encode as 16-bit PCM (scale 32768, the inverse of `parse`). Out-of-range
/// samples are clamped.
pub fn encode_pcm16(sample_rate: u32, channels: u16, samples: &[f32]) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
        let v = (s * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

pub fn read_file(path: &std::path::Path) -> Result<Wav, Box<dyn std::error::Error>> {
    let meta = std::fs::metadata(path)?;
    if meta.len() as usize > MAX_WAV_BYTES {
        return Err(Box::new(WavError::TooLarge));
    }
    Ok(parse(&std::fs::read(path)?)?)
}
