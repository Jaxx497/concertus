use anyhow::{Context, Result, anyhow};
use byteorder::{LittleEndian, ReadBytesExt};
use std::{io::Cursor, path::Path, process::Command, time::Duration};

const WF_LEN: usize = 500;
const MIN_SAMPLES_PER_POINT: usize = 200; // Minimum for short files
const MAX_SAMPLES_PER_POINT: usize = 5000; // Maximum for very long files
const SMOOTHING_FACTOR: f32 = 0.2;

/// Generate a waveform using ffmpeg by piping output directly to memory
pub fn generate_waveform<P: AsRef<Path>>(audio_path: P) -> Vec<f32> {
    let path = audio_path.as_ref();

    // TODO: Handle bad waveform data
    match extract_waveform_data(path) {
        Ok(waveform) => waveform,
        Err(_) => {
            vec![0.2; WF_LEN] // Return a flat line if all fails
        }
    }
}

/// Extract duration from audio file using ffmpeg
fn get_audio_duration<P: AsRef<Path>>(audio_path: P) -> Result<Duration> {
    let audio_path_str = audio_path
        .as_ref()
        .to_str()
        .ok_or_else(|| anyhow!("Audio path contains invalid Unicode"))?;

    // Use ffprobe to get duration
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            audio_path_str,
        ])
        .output()
        .context("Failed to execute ffprobe")?;

    if !output.status.success() {
        return Err(anyhow!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let duration_secs = duration_str
        .parse::<f64>()
        .context("Failed to parse duration")?;

    Ok(Duration::from_secs_f64(duration_secs))
}

/// Extract waveform data from audio file
fn extract_waveform_data<P: AsRef<Path>>(audio_path: P) -> Result<Vec<f32>> {
    // Get audio duration to calculate optimal sampling
    let duration = match get_audio_duration(&audio_path) {
        Ok(d) => d,
        Err(_) => {
            return Err(anyhow!("Could not determine audio length"));
        }
    };

    // Calculate adaptive samples per point based on duration
    let samples_per_point = calculate_adaptive_samples(duration);

    // Get the path as string, with better error handling
    let audio_path_str = audio_path
        .as_ref()
        .to_str()
        .ok_or_else(|| anyhow!("Audio path contains invalid Unicode"))?;

    // Create a process to pipe audio data directly to memory using ffmpeg
    let mut cmd = Command::new("ffmpeg");
    let output = cmd
        .args(&[
            "-i",
            audio_path_str,
            "-ac",
            "1", // Convert to mono
            "-ar",
            "22050", // Maintain resolution, half as many datapoints
            // "44100",
            "-af",
            "dynaudnorm=f=500:g=31,highpass=f=350,volume=2,bass=gain=-8:frequency=200,treble=gain=10:frequency=6000", // I wish I could explain this, but this is the best we're gonna get without having a masters in audio engineering
            "-loglevel",
            "warning",
            "-f",
            "f32le",
            "-",
        ])
        .output()
        .context("Failed to execute ffmpeg. Is it installed and in your PATH?")?;

    // Check for errors
    if !output.status.success() {
        return Err(anyhow!(
            "FFmpeg conversion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let pcm_data = output.stdout;
    let mut waveform = process_pcm_to_waveform(&pcm_data, samples_per_point)?;

    smooth_waveform(&mut waveform);
    normalize_waveform(&mut waveform);

    Ok(waveform)
}

/// Calculate adaptive samples per point based on duration
fn calculate_adaptive_samples(duration: Duration) -> usize {
    let duration_secs = duration.as_secs_f32();
    let sample_rate = 44100.0; // Standard sample rate

    // Calculate total samples in the file
    let total_samples = (duration_secs * sample_rate) as usize;

    // Calculate base samples per point
    // This ensures we consider at least ~10% of the audio total
    let ideal_samples = total_samples / (WF_LEN * 10);

    // Clamp between min and max values
    ideal_samples.clamp(MIN_SAMPLES_PER_POINT, MAX_SAMPLES_PER_POINT)
}

/// Process raw PCM float data into a vector of f32 values
fn process_pcm_to_waveform(pcm_data: &[u8], samples_per_point: usize) -> Result<Vec<f32>> {
    // Create a cursor to read the PCM data as 32-bit floats
    let mut cursor = Cursor::new(pcm_data);

    let total_samples = pcm_data.len() / 4; // Each float is 4 bytes

    // If the file is very short, we might need to adapt our approach
    if total_samples < WF_LEN * samples_per_point {
        return process_short_pcm(pcm_data);
    }

    let sample_step = total_samples / WF_LEN;
    let mut waveform = Vec::with_capacity(WF_LEN);

    for i in 0..WF_LEN {
        let position = i * sample_step * 4; // 4 bytes per float
        if position >= pcm_data.len() {
            break;
        }

        cursor.set_position(position as u64);
        let mut sum_squares = 0.0;
        let mut samples_read = 0;
        let mut max_value = 0.0f32;

        let max_samples = samples_per_point.min(sample_step);
        for _ in 0..max_samples {
            if cursor.position() >= pcm_data.len() as u64 {
                break;
            }

            match cursor.read_f32::<LittleEndian>() {
                Ok(sample) => {
                    // Track maximum absolute value
                    let abs_sample = sample.abs();
                    if abs_sample > max_value {
                        max_value = abs_sample;
                    }

                    // Sum squares for RMS calculation
                    sum_squares += sample * sample;
                    samples_read += 1;
                }
                Err(_) => break,
            }
        }

        match samples_read > 0 {
            true => {
                let rms = (sum_squares / samples_read as f32).sqrt();
                let value = rms.min(1.0);
                waveform.push(value);
            }
            false => waveform.push(0.0),
        }
    }

    // Fill additional values if necessary
    while waveform.len() < WF_LEN {
        waveform.push(0.0);
    }

    Ok(waveform)
}

/// Process very short PCM files
fn process_short_pcm(pcm_data: &[u8]) -> Result<Vec<f32>> {
    let mut cursor = Cursor::new(pcm_data);
    let total_samples = pcm_data.len() / 4;

    // For very short files, we'll divide the available samples evenly
    let samples_per_section = total_samples / WF_LEN.max(1);
    let extra_samples = total_samples % WF_LEN;

    let mut waveform = Vec::with_capacity(WF_LEN);
    let mut position = 0;

    for i in 0..WF_LEN {
        // Calculate how many samples this section should have
        let samples_this_section = if i < extra_samples {
            samples_per_section + 1
        } else {
            samples_per_section
        };

        if samples_this_section == 0 {
            waveform.push(0.0);
            continue;
        }

        cursor.set_position((position * 4) as u64);

        let mut sum_squares = 0.0;
        let mut max_value = 0.0f32;
        let mut samples_read = 0;

        for _ in 0..samples_this_section {
            if cursor.position() >= pcm_data.len() as u64 {
                break;
            }

            match cursor.read_f32::<LittleEndian>() {
                Ok(sample) => {
                    let abs_sample = sample.abs();
                    if abs_sample > max_value {
                        max_value = abs_sample;
                    }
                    sum_squares += sample * sample;
                    samples_read += 1;
                }
                Err(_) => break,
            }
        }

        position += samples_this_section;

        if samples_read > 0 {
            let rms = (sum_squares / samples_read as f32).sqrt();
            //FIXME:  let value = (rms * 0.8 + max_value * 0.2).min(1.0);
            let value = rms.min(1.0);
            waveform.push(value);
        } else {
            waveform.push(0.0);
        }
    }

    while waveform.len() < WF_LEN {
        waveform.push(0.0);
    }

    Ok(waveform)
}

/// Apply a smoothing filter to the waveform with float smoothing factor
fn smooth_waveform(waveform: &mut Vec<f32>) {
    let smoothing_factor = SMOOTHING_FACTOR;
    if waveform.len() <= (smoothing_factor.ceil() as usize * 2 + 1) {
        return; // Not enough points to smooth
    }

    let original = waveform.clone();
    let range = smoothing_factor.ceil() as isize;

    for i in 0..waveform.len() {
        let mut sum = 0.0;
        let mut total_weight = 0.0;

        // Calculate weighted average of surrounding points
        for offset in -range..=range {
            let idx = i as isize + offset;
            if idx >= 0 && idx < original.len() as isize {
                // Weight calculation - based on distance and the smoothing factor
                // Points beyond the float smoothing factor get reduced weight
                let distance = offset.abs() as f32;
                let weight = if distance <= smoothing_factor {
                    // Full weight for points within the smooth factor
                    1.0
                } else {
                    // Partial weight for the fractional part
                    1.0 - (distance - smoothing_factor)
                };

                if weight > 0.0 {
                    sum += original[idx as usize] * weight;
                    total_weight += weight;
                }
            }
        }

        if total_weight > 0.0 {
            waveform[i] = sum / total_weight;
        }
    }
}

/// Normalize the waveform to a 0.0-1.0 range with improved dynamics
fn normalize_waveform(waveform: &mut [f32]) {
    if waveform.is_empty() {
        return;
    }

    let min = *waveform
        .iter()
        .min_by(|a, b| a.total_cmp(b))
        .unwrap_or(&0.0);

    let max = *waveform
        .iter()
        .max_by(|a, b| a.total_cmp(b))
        .unwrap_or(&1.0);

    if (max - min).abs() < f32::EPSILON {
        for value in waveform.iter_mut() {
            *value = 0.3;
        }
    } else {
        for value in waveform.iter_mut() {
            *value = (*value - min) / (max - min);
        }
    }
}
