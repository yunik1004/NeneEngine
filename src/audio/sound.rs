use std::path::Path;

use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
    meta::MetadataOptions, probe::Hint,
};

/// A decoded audio clip loaded into memory.
pub struct Sound {
    pub(crate) samples: Vec<f32>,
    pub(crate) channels: usize,
    pub(crate) sample_rate: u32,
}

impl Sound {
    /// Load an audio file from disk. Supports any format supported by Symphonia
    /// (mp3, ogg, flac, wav, …).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| format!("Failed to probe audio format: {e}"))?;

        let mut format = probed.format;
        let track = format.default_track().ok_or("No audio track found")?;
        let track_id = track.id;
        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or("Unknown sample rate")?;
        let channels = track
            .codec_params
            .channels
            .ok_or("Unknown channel count")?
            .count();

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Failed to create decoder: {e}"))?;

        let mut samples = Vec::new();
        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track_id {
                continue;
            }
            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let spec = *decoded.spec();
            let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
            buf.copy_interleaved_ref(decoded);
            samples.extend_from_slice(buf.samples());
        }

        Ok(Self {
            samples,
            channels,
            sample_rate,
        })
    }

    /// Decode an audio clip from a raw byte buffer.
    ///
    /// `ext_hint` is the file-extension hint passed to Symphonia's prober
    /// (e.g. `"ogg"`, `"mp3"`, `"wav"`). It is used only to help the prober
    /// pick the right format; it may be an empty string if unknown.
    pub fn from_bytes(
        bytes: Vec<u8>,
        ext_hint: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cursor = std::io::Cursor::new(bytes);
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

        let mut hint = Hint::new();
        if !ext_hint.is_empty() {
            hint.with_extension(ext_hint);
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| format!("Failed to probe audio format: {e}"))?;

        let mut format = probed.format;
        let track = format.default_track().ok_or("No audio track found")?;
        let track_id = track.id;
        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or("Unknown sample rate")?;
        let channels = track
            .codec_params
            .channels
            .ok_or("Unknown channel count")?
            .count();

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Failed to create decoder: {e}"))?;

        let mut samples = Vec::new();
        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track_id {
                continue;
            }
            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let spec = *decoded.spec();
            let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
            buf.copy_interleaved_ref(decoded);
            samples.extend_from_slice(buf.samples());
        }

        Ok(Self { samples, channels, sample_rate })
    }

    /// Generate a sine-wave tone entirely in memory — no file I/O.
    pub fn sine_wave(freq: f32, duration: f32, sample_rate: u32) -> Self {
        let n = (sample_rate as f32 * duration) as usize;
        let samples = (0..n)
            .map(|i| (i as f32 / sample_rate as f32 * freq * std::f32::consts::TAU).sin())
            .collect();
        Self {
            samples,
            channels: 1,
            sample_rate,
        }
    }

    pub fn channels(&self) -> usize {
        self.channels
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn sample_count(&self) -> usize {
        self.samples.len() / self.channels
    }
}
