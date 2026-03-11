use anyhow::Result;
use rustfft::{num_complex::Complex, FftPlanner};
use std::fs;
use std::path::Path;

use super::wav_metadata;
use super::{Confidence, Signal, SignalBuilder, SignalSource};

const FFT_SIZE: usize = 2048;
const MAX_FRAMES: usize = 64;
const BANDWIDTH_THRESHOLD: f64 = 0.7;
const CUTOFF_ENERGY_RATIO: f64 = 0.02;

fn decode_pcm_16le(data: &[u8], channels: u16) -> Vec<f64> {
    let bytes_per_sample = 2usize;
    let block_align = bytes_per_sample * channels as usize;
    let num_blocks = data.len() / block_align;
    let mut samples = Vec::with_capacity(num_blocks);
    for i in 0..num_blocks {
        let offset = i * block_align;
        if offset + 2 > data.len() {
            break;
        }
        let raw = i16::from_le_bytes([data[offset], data[offset + 1]]);
        samples.push(raw as f64 / 32768.0);
    }
    samples
}

fn compute_avg_spectrum(samples: &[f64], fft_size: usize) -> Vec<f64> {
    if samples.len() < fft_size {
        return vec![];
    }
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mid = samples.len() / 2;
    let half_window = (MAX_FRAMES * fft_size) / 2;
    let start = mid.saturating_sub(half_window);
    let available = &samples[start..];
    let num_bins = fft_size / 2;
    let mut avg_power = vec![0.0f64; num_bins];
    let mut frame_count = 0usize;
    let hop = fft_size / 2;
    let mut pos = 0;
    while pos + fft_size <= available.len() && frame_count < MAX_FRAMES {
        let mut buffer: Vec<Complex<f64>> = available[pos..pos + fft_size]
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let w = 0.5
                    * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (fft_size - 1) as f64).cos());
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

fn find_bandwidth_cutoff(spectrum: &[f64], sample_rate: u32) -> Option<(f64, f64)> {
    if spectrum.is_empty() {
        return None;
    }
    let num_bins = spectrum.len();
    let nyquist = sample_rate as f64 / 2.0;
    let bin_hz = nyquist / num_bins as f64;
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

fn spectral_flatness(spectrum: &[f64]) -> f64 {
    let n = spectrum.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
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

pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let data = fs::read(path)?;
    let wav = match wav_metadata::parse_wav_full(&data) {
        Some(w) => w,
        None => return Ok(vec![]),
    };
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

    if let Some((cutoff_freq, bandwidth_ratio)) =
        find_bandwidth_cutoff(&spectrum, wav.fmt.sample_rate)
    {
        let nyquist = wav.fmt.sample_rate as f64 / 2.0;
        signals.push(
            SignalBuilder::new(
                SignalSource::AudioSpectral,
                Confidence::Low,
                "signal_audio_cutoff",
            )
            .param("freq", format!("{:.0}", cutoff_freq))
            .param("pct", format!("{:.0}", bandwidth_ratio * 100.0))
            .param("nyquist", format!("{:.0}", nyquist))
            .detail("cutoff_frequency", format!("{:.0}Hz", cutoff_freq))
            .detail("nyquist", format!("{:.0}Hz", nyquist))
            .detail("bandwidth_used", format!("{:.1}%", bandwidth_ratio * 100.0))
            .build(),
        );
    }

    let flatness = spectral_flatness(&spectrum);
    let nyquist = wav.fmt.sample_rate as f64 / 2.0;
    if nyquist <= 12000.0 && wav.fmt.channels == 1 && flatness < 0.05 {
        signals.push(
            SignalBuilder::new(
                SignalSource::AudioSpectral,
                Confidence::Low,
                "signal_audio_flatness",
            )
            .param("value", format!("{:.4}", flatness))
            .detail("spectral_flatness", format!("{:.4}", flatness))
            .build(),
        );
    }

    Ok(signals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_pcm_16le() {
        let data = vec![0u8; 200];
        let samples = decode_pcm_16le(&data, 1);
        assert_eq!(samples.len(), 100);
        assert!(samples.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_decode_pcm_16le_stereo() {
        let data = vec![0u8; 400];
        let samples = decode_pcm_16le(&data, 2);
        assert_eq!(samples.len(), 100);
    }

    #[test]
    fn test_spectral_flatness_pure_tone() {
        let mut spectrum = vec![0.0; 1024];
        spectrum[100] = 1.0;
        let flatness = spectral_flatness(&spectrum);
        assert!(flatness <= 1.0);
    }

    #[test]
    fn test_spectral_flatness_white_noise() {
        let spectrum = vec![1.0; 1024];
        let flatness = spectral_flatness(&spectrum);
        assert!((flatness - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_find_bandwidth_cutoff_full() {
        let spectrum = vec![1.0; 1024];
        let result = find_bandwidth_cutoff(&spectrum, 48000);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_bandwidth_cutoff_half() {
        let mut spectrum = vec![0.0; 1024];
        for i in 0..300 {
            spectrum[i] = 1.0;
        }
        let result = find_bandwidth_cutoff(&spectrum, 48000);
        assert!(result.is_some());
        let (freq, ratio) = result.unwrap();
        assert!(freq < 12000.0);
        assert!(ratio < BANDWIDTH_THRESHOLD);
    }

    #[test]
    fn test_compute_avg_spectrum_silence() {
        let samples = vec![0.0; FFT_SIZE * 4];
        let spectrum = compute_avg_spectrum(&samples, FFT_SIZE);
        assert!(!spectrum.is_empty());
        assert!(spectrum.iter().all(|&x| x < 1e-10));
    }

    #[test]
    fn test_compute_avg_spectrum_too_short() {
        let samples = vec![0.0; 100];
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
