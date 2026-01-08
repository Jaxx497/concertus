use anyhow::{Context, Result, anyhow, bail};
use crossbeam_channel::Receiver;
use std::{
    io::{Cursor, Read},
    path::Path,
    process::Command,
    sync::Arc,
    thread,
    time::Duration,
};

use crate::{
    key_handler::Incrementor,
    library::{SimpleSong, SongDatabase},
    ui_state::UiState,
};

const WF_LEN: usize = 500;
static WAVEFORM_STEP: f32 = 0.5;
const MIN_SAMPLES_PER_POINT: usize = 200; // Minimum for short files
const MAX_SAMPLES_PER_POINT: usize = 4000; // Maximum for very long files

#[derive(PartialEq)]
pub enum WaveformState {
    None,
    Loading,
    Ready(Vec<f32>),
    Failed,
}

pub struct WaveformManager {
    state: WaveformState,
    smoothed_view: Vec<f32>,
    smoothing_factor: f32,
    reciever: Option<Receiver<Result<Vec<f32>>>>,
}

impl WaveformManager {
    pub fn new() -> Self {
        WaveformManager {
            state: WaveformState::None,
            smoothed_view: Vec::with_capacity(WF_LEN),
            smoothing_factor: 1.0,
            reciever: None,
        }
    }

    pub fn request(&mut self, song: &SimpleSong) {
        if let Ok(cached) = song.get_waveform() {
            self.state = WaveformState::Ready(cached);
            self.apply_smoothing();
            return;
        }

        if let Ok(path) = song.get_path() {
            let (tx, rx) = crossbeam_channel::bounded(1);
            self.state = WaveformState::Loading;

            thread::spawn(move || {
                let res = generate_waveform(path);
                let _ = tx.send(res);
            });

            self.reciever = Some(rx)
        }
    }

    pub fn reciever(&self) -> Option<&Receiver<Result<Vec<f32>>>> {
        self.reciever.as_ref()
    }

    pub fn complete(&mut self, result: Result<Vec<f32>>, song: Option<&Arc<SimpleSong>>) {
        match result {
            Ok(waveform) => {
                if let Some(s) = song {
                    let _ = s.set_waveform_db(&waveform);
                }
                self.state = WaveformState::Ready(waveform);
                self.apply_smoothing();
            }
            Err(_) => self.state = WaveformState::Failed,
        }
        self.reciever = None;
    }
}

impl WaveformManager {
    pub fn clear(&mut self) {
        self.reciever = None;
        self.smoothed_view.clear();
        self.state = WaveformState::None;
    }

    pub fn apply_smoothing(&mut self) {
        if let WaveformState::Ready(raw) = &mut self.state {
            self.smoothed_view = smooth_waveform(raw, self.smoothing_factor);
        }
    }

    pub fn increment_smoothness(&mut self, direction: Incrementor) {
        match direction {
            Incrementor::Up => {
                if self.smoothing_factor < 3.9 {
                    self.smoothing_factor += WAVEFORM_STEP;
                    self.apply_smoothing();
                }
            }
            Incrementor::Down => {
                if self.smoothing_factor > 0.1 {
                    self.smoothing_factor -= WAVEFORM_STEP;
                    self.apply_smoothing();
                }
            }
        }
    }

    fn get_waveform_visual(&self) -> &[f32] {
        self.smoothed_view.as_slice()
    }
}

impl UiState {
    pub fn request_waveform(&mut self, song: &SimpleSong) {
        self.waveform.request(song);
    }

    pub fn handle_wf_result(&mut self, result: Result<Vec<f32>>, song: Option<&Arc<SimpleSong>>) {
        self.waveform.complete(result, song);
    }

    pub fn get_waveform_state(&self) -> &WaveformState {
        &self.waveform.state
    }

    pub fn wf_reciever(&self) -> Option<&Receiver<Result<Vec<f32>>>> {
        self.waveform.reciever()
    }

    pub fn clear_waveform(&mut self) {
        self.waveform.clear();
    }

    pub fn get_waveform_as_slice(&self) -> &[f32] {
        self.waveform.get_waveform_visual()
    }

    pub fn get_smoothing_factor(&self) -> f32 {
        self.waveform.smoothing_factor
    }

    pub fn set_smoothing_factor(&mut self, sf: f32) {
        self.waveform.smoothing_factor = sf
    }

    pub fn increment_wf_smoothness(&mut self, direction: Incrementor) {
        self.waveform.increment_smoothness(direction);
    }
}

/// Generate a waveform using ffmpeg by piping output directly to memory
pub fn generate_waveform<P: AsRef<Path>>(audio_path: P) -> Result<Vec<f32>> {
    let path = audio_path.as_ref();
    extract_waveform_data(path)
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
        bail!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
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
            bail!("Could not determine audio length");
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
            "22050",
            "-af",
            "dynaudnorm=f=500:g=31,highpass=f=350,volume=2,bass=gain=-8:frequency=200,treble=gain=10:frequency=6000", // I wish I could explain this, but this is the best we're gonna get without having a masters in audio engineering
            "-loglevel",
            "warning",
            "-f",
            "f32le",
            "-",
        ])
        .output()
        .context("Failed to execute ffmpeg. Is it installed and in your `PATH`?")?;

    // Check for errors
    if !output.status.success() {
        bail!(
            "FFmpeg conversion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let pcm_data = output.stdout;
    let mut waveform = process_pcm_to_waveform(&pcm_data, samples_per_point)?;

    normalize_waveform(&mut waveform);

    Ok(waveform)
}

/// Calculate adaptive samples per point based on duration
fn calculate_adaptive_samples(duration: Duration) -> usize {
    let duration_secs = duration.as_secs_f32();
    let sample_rate = 44100.0; // Standard sample rate

    let total_samples = (duration_secs * sample_rate) as usize;
    let ideal_samples = total_samples / (WF_LEN * 10);

    ideal_samples.clamp(MIN_SAMPLES_PER_POINT, MAX_SAMPLES_PER_POINT)
}

/// Process raw PCM float data into a vector of f32 values
fn process_pcm_to_waveform(pcm_data: &[u8], samples_per_point: usize) -> Result<Vec<f32>> {
    let mut cursor = Cursor::new(pcm_data);

    let total_samples = pcm_data.len() / 4;

    // If the file is very short, adapt the approach
    if total_samples < WF_LEN * samples_per_point {
        return process_short_pcm(pcm_data);
    }

    let sample_step = total_samples / WF_LEN;
    let mut waveform = Vec::with_capacity(WF_LEN);

    for i in 0..WF_LEN {
        let position = i * sample_step * 4;
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

            let mut bytes = [0u8; 4];
            match cursor.read_exact(&mut bytes) {
                Ok(_) => {
                    let sample = f32::from_le_bytes(bytes);
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

            let mut bytes = [0u8; 4];
            match cursor.read_exact(&mut bytes) {
                Ok(_) => {
                    let sample = f32::from_le_bytes(bytes);
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

        match samples_read > 0 {
            true => {
                let rms = (sum_squares / samples_read as f32).sqrt();
                let value = rms.min(1.0);
                waveform.push(value);
            }
            false => waveform.push(0.0),
        }
    }

    while waveform.len() < WF_LEN {
        waveform.push(0.0);
    }

    Ok(waveform)
}

/// Apply a smoothing filter to the waveform with float smoothing factor
pub fn smooth_waveform(waveform: &[f32], smoothing_factor: f32) -> Vec<f32> {
    if waveform.len() <= (smoothing_factor.ceil() as usize * 2 + 1) {
        return waveform.to_vec();
    }

    let range = smoothing_factor.ceil() as isize;

    waveform
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let mut sum = 0.0;
            let mut total_weight = 0.0;

            // Calculate weighted average of surrounding points
            for offset in -range..=range {
                let idx = i as isize + offset;
                if idx >= 0 && idx < waveform.len() as isize {
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
                        sum += waveform[idx as usize] * weight;
                        total_weight += weight;
                    }
                }
            }

            if total_weight > 0.0 {
                sum / total_weight
            } else {
                waveform[i]
            }
        })
        .collect()
}

/// Normalize the waveform to a 0.0-1.0 range
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

    match (max - min).abs() < f32::EPSILON {
        true => waveform.iter_mut().for_each(|value| *value = 0.3),
        false => waveform
            .iter_mut()
            .for_each(|value| *value = (*value - min) / (max - min)),
    }
}
