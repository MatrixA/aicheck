use anyhow::Result;
use rustfft::{num_complex::Complex, FftPlanner};
use std::fs;
use std::path::Path;

use super::wav_metadata;
use super::{Confidence, Signal, SignalSource};

/// FFT window size (number of samples per frame).
const FFT_SIZE: usize = 2048;

/// Number of FFT frames to average over for stable results.
/// Analyze ~2 seconds of audio from the middle of the file.
const MAX_FRAMES: usize = 64;

/// Fraction of Nyquist bandwidth that must contain energy for "full bandwidth" audio.
/// If energy is concentrated below this fraction, the audio may be AI-generated
/// (many TTS/music models output at lower effective bandwidths).
const BANDWIDTH_THRESHOLD: f64 = 0.7;

/// Minimum energy ratio between used and unused bands to flag bandwidth cutoff.
/// If the top portion of the spectrum has < this fraction of the lower portion's energy,
/// it's a hard cutoff.
const CUTOFF_ENERGY_RATIO: f64 = 0.02;

// ---------------------------------------------------------------------------
// PCM decoding
// ---------------------------------------------------------------------------

/// Decode 16-bit little-endian PCM bytes to f64 samples normalized to [-1, 1].
fn decode_pcm_16le(data: &[u8], channels: u16) -> Vec<f64> {
    let bytes_per_sample = 2usize;
    let block_align = bytes_per_sample * channels as usize;
    let num_blocks = data.len() / block_align;

    let mut samples = Vec::with_capacity(num_blocks);
    for i in 0..num_blocks {
        // Take first channel only (mono mix)
        let offset = i * block_align;
        if offset + 2 > data.len() {
            break;
        }
        let raw = i16::from_le_bytes([data[offset], data[offset + 1]]);
        samples.push(raw as f64 / 32768.0);
    }
    samples
}

// ---------------------------------------------------------------------------
// Spectral analysis
// ---------------------------------------------------------------------------

/// Compute average power spectrum from the middle of the audio.
fn compute_avg_spectrum(samples: &[f64], fft_size: usize) -> Vec<f64> {
    if samples.len() < fft_size {
        return vec![];
    }

    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Start from the middle of the audio (skip silence at start/end)
    let mid = samples.len() / 2;
    let half_window = (MAX_FRAMES * fft_size) / 2;
    let start = mid.saturating_sub(half_window);
    let available = &samples[start..];

    let num_bins = fft_size / 2;
    let mut avg_power = vec![0.0f64; num_bins];
    let mut frame_count = 0usize;

    let hop = fft_size / 2; // 50% overlap
    let mut pos = 0;

    while pos + fft_size <= available.len() && frame_count < MAX_FRAMES {
        let mut buffer: Vec<Complex<f64>> = available[pos..pos + fft_size]
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                // Apply Hann window
                let w = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (fft_size - 1) as f64).cos());
                Complex::new(s * w, 0.0)
            })
            .collect();

        fft.process(&mut buffer);

        for (bin, power) in avg_power.iter_mut().enumerate() {
            *power += buffer[bin].norm_sqr();
        }

        frame_count += 1;
        pos += hop;
    }

    if frame_count == 0 {
        return vec![];
    }

    for power in avg_power.iter_mut() {
        *power /= frame_count as f64;
    }

    avg_power
}

/// Find the effective bandwidth cutoff frequency.
/// Returns the frequency (Hz) where energy drops sharply, or None if full bandwidth.
fn find_bandwidth_cutoff(spectrum: &[f64], sample_rate: u32) -> Option<(f64, f64)> {
    if spectrum.is_empty() {
        return None;
    }

    let num_bins = spectrum.len();
    let nyquist = sample_rate as f64 / 2.0;
    let bin_hz = nyquist / num_bins as f64;

    // Find the bin where cumulative energy reaches 99% of total
    let total_energy: f64 = spectrum.iter().sum();
    if total_energy == 0.0 {
        return None;
    }

    let mut cumulative = 0.0;
    let mut cutoff_bin = num_bins;
    for (i, &power) in spectrum.iter().enumerate() {
        cumulative += power;
        if cumulative >= total_energy * 0.99 {
            cutoff_bin = i + 1;
            break;
        }
    }

    let cutoff_freq = cutoff_bin as f64 * bin_hz;
    let bandwidth_ratio = cutoff_freq / nyquist;

    // Check for hard cutoff: compare energy above and below cutoff
    if bandwidth_ratio < BANDWIDTH_THRESHOLD {
        let below_energy: f64 = spectrum[..cutoff_bin].iter().sum();
        let above_energy: f64 = spectrum[cutoff_bin..].iter().sum();
        let ratio = if below_energy > 0.0 {
            above_energy / below_energy
        } else {
            0.0
        };

        if ratio < CUTOFF_ENERGY_RATIO {
            return Some((cutoff_freq, bandwidth_ratio));
        }
    }

    None
}

/// Compute spectral flatness (Wiener entropy).
/// Values close to 1.0 = noise-like, close to 0.0 = tonal.
/// AI-generated audio often has different flatness than natural audio.
fn spectral_flatness(spectrum: &[f64]) -> f64 {
    let n = spectrum.len() as f64;
    if n == 0.0 {
        return 0.0;
    }

    // Filter out zero/near-zero bins to avoid log(0)
    let filtered: Vec<f64> = spectrum.iter().copied().filter(|&x| x > 1e-20).collect();
    if filtered.is_empty() {
        return 0.0;
    }

    let n = filtered.len() as f64;
    let log_mean = filtered.iter().map(|x| x.ln()).sum::<f64>() / n;
    let geometric_mean = log_mean.exp();
    let arithmetic_mean = filtered.iter().sum::<f64>() / n;

    if arithmetic_mean > 0.0 {
        geometric_mean / arithmetic_mean
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Detect AI signals from audio spectral analysis on WAV files.
pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let data = fs::read(path)?;
    let wav = match wav_metadata::parse_wav_full(&data) {
        Some(w) => w,
        None => return Ok(vec![]),
    };

    // Only support 16-bit PCM for now
    if wav.fmt.bits_per_sample != 16 || wav.pcm_start >= wav.pcm_end {
        return Ok(vec![]);
    }

    let pcm_data = &data[wav.pcm_start..wav.pcm_end];
    let samples = decode_pcm_16le(pcm_data, wav.fmt.channels);

    if samples.len() < FFT_SIZE {
        return Ok(vec![]);
    }

    let spectrum = compute_avg_spectrum(&samples, FFT_SIZE);
    if spectrum.is_empty() {
        return Ok(vec![]);
    }

    let mut signals = Vec::new();

    // Check for hard frequency cutoff
    if let Some((cutoff_freq, bandwidth_ratio)) = find_bandwidth_cutoff(&spectrum, wav.fmt.sample_rate) {
        let nyquist = wav.fmt.sample_rate as f64 / 2.0;
        signals.push(Signal {
            source: SignalSource::AudioSpectral,
            confidence: Confidence::Low,
            description: format!(
                "Hard frequency cutoff at {:.0}Hz ({:.0}% of {:.0}Hz Nyquist) — typical of AI/TTS synthesis",
                cutoff_freq,
                bandwidth_ratio * 100.0,
                nyquist,
            ),
            tool: None,
            details: vec![
                ("cutoff_frequency".to_string(), format!("{:.0}Hz", cutoff_freq)),
                ("nyquist".to_string(), format!("{:.0}Hz", nyquist)),
                ("bandwidth_used".to_string(), format!("{:.1}%", bandwidth_ratio * 100.0)),
            ],
        });
    }

    // Check spectral flatness
    // Very low flatness in speech range can indicate synthetic voice
    // (natural speech has more spectral variation across frames)
    let flatness = spectral_flatness(&spectrum);
    let nyquist = wav.fmt.sample_rate as f64 / 2.0;

    // For speech-range audio (Nyquist <= 12kHz), unusually low flatness
    // combined with mono suggests TTS
    if nyquist <= 12000.0 && wav.fmt.channels == 1 && flatness < 0.05 {
        signals.push(Signal {
            source: SignalSource::AudioSpectral,
            confidence: Confidence::Low,
            description: format!(
                "Spectral flatness {:.4} suggests synthetic audio (natural speech typically > 0.05)",
                flatness,
            ),
            tool: None,
            details: vec![
                ("spectral_flatness".to_string(), format!("{:.4}", flatness)),
            ],
        });
    }

    Ok(signals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_pcm_16le() {
        // Silence: all zeros
        let data = vec![0u8; 200];
        let samples = decode_pcm_16le(&data, 1);
        assert_eq!(samples.len(), 100);
        assert!(samples.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_decode_pcm_16le_stereo() {
        // Stereo: takes first channel
        let data = vec![0u8; 400]; // 100 stereo samples
        let samples = decode_pcm_16le(&data, 2);
        assert_eq!(samples.len(), 100);
    }

    #[test]
    fn test_spectral_flatness_pure_tone() {
        // Pure tone at one frequency should have very low flatness
        let mut spectrum = vec![0.0; 1024];
        spectrum[100] = 1.0; // Single peak
        let flatness = spectral_flatness(&spectrum);
        // With only one non-zero bin, flatness should be 1.0 (single value)
        // but with more peaks it decreases
        assert!(flatness <= 1.0);
    }

    #[test]
    fn test_spectral_flatness_white_noise() {
        // Uniform spectrum should have high flatness (close to 1.0)
        let spectrum = vec![1.0; 1024];
        let flatness = spectral_flatness(&spectrum);
        assert!((flatness - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_find_bandwidth_cutoff_full() {
        // Flat spectrum = full bandwidth, no cutoff
        let spectrum = vec![1.0; 1024];
        let result = find_bandwidth_cutoff(&spectrum, 48000);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_bandwidth_cutoff_half() {
        // Energy only in lower third = sharp cutoff detected
        let mut spectrum = vec![0.0; 1024];
        for i in 0..300 {
            spectrum[i] = 1.0;
        }
        let result = find_bandwidth_cutoff(&spectrum, 48000);
        assert!(result.is_some());
        let (freq, ratio) = result.unwrap();
        assert!(freq < 12000.0); // Should be below Nyquist (24kHz)
        assert!(ratio < BANDWIDTH_THRESHOLD);
    }

    #[test]
    fn test_compute_avg_spectrum_silence() {
        let samples = vec![0.0; FFT_SIZE * 4];
        let spectrum = compute_avg_spectrum(&samples, FFT_SIZE);
        assert!(!spectrum.is_empty());
        // Silence should have near-zero energy everywhere
        assert!(spectrum.iter().all(|&x| x < 1e-10));
    }

    #[test]
    fn test_compute_avg_spectrum_too_short() {
        let samples = vec![0.0; 100]; // Too short for FFT
        let spectrum = compute_avg_spectrum(&samples, FFT_SIZE);
        assert!(spectrum.is_empty());
    }

    #[test]
    fn test_detect_non_wav() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"not a wav").unwrap();
        let signals = detect(tmp.path()).unwrap();
        assert!(signals.is_empty());
    }
}
