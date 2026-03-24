use std::path::Path;
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

pub struct Sound {
    pub(crate) samples: Vec<f32>,
    pub(crate) channels: usize,
    pub(crate) sample_rate: u32,
}

impl Sound {
    pub fn load(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let file = std::fs::File::open(path).expect("Failed to open audio file");
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
            .expect("Failed to probe audio format");

        let mut format = probed.format;
        let track = format.default_track().expect("No audio track found");
        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.expect("Unknown sample rate");
        let channels = track
            .codec_params
            .channels
            .expect("Unknown channel count")
            .count();

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .expect("Failed to create decoder");

        let mut samples = Vec::new();

        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(Error::IoError(_)) => break,
                Err(_) => break,
            };
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

        Self {
            samples,
            channels,
            sample_rate,
        }
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 전체 프레임 수 (채널 수와 무관하게 시간 단위)
    pub fn sample_count(&self) -> usize {
        self.samples.len() / self.channels
    }
}
