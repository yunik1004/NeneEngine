use super::Sound;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    Arc,
    mpsc::{self, Sender},
};

pub struct AudioDevice {
    _stream: cpal::Stream,
    sender: Sender<Arc<Sound>>,
}

struct Playback {
    sound: Arc<Sound>,
    position: f64,
}

impl AudioDevice {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device found");
        let config = device.default_output_config().expect("No output config");

        let sample_rate = config.sample_rate();
        let channels = config.channels() as usize;

        let (sender, receiver) = mpsc::channel::<Arc<Sound>>();
        let mut playbacks: Vec<Playback> = Vec::new();

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    while let Ok(sound) = receiver.try_recv() {
                        playbacks.push(Playback {
                            sound,
                            position: 0.0,
                        });
                    }

                    for s in data.iter_mut() {
                        *s = 0.0;
                    }

                    playbacks.retain_mut(|pb| {
                        let ratio = pb.sound.sample_rate as f64 / sample_rate as f64;
                        let src_channels = pb.sound.channels;

                        for frame in data.chunks_mut(channels) {
                            let src_frame = pb.position as usize;
                            let src_idx = src_frame * src_channels;

                            if src_idx + src_channels > pb.sound.samples.len() {
                                return false;
                            }

                            for (ch, out) in frame.iter_mut().enumerate() {
                                let src_ch = ch.min(src_channels - 1);
                                *out += pb.sound.samples[src_idx + src_ch];
                            }

                            pb.position += ratio;
                        }
                        true
                    });
                },
                |err| eprintln!("Audio stream error: {err}"),
                None,
            )
            .expect("Failed to build output stream");

        stream.play().expect("Failed to start audio stream");

        Self {
            _stream: stream,
            sender,
        }
    }

    pub fn play(&self, sound: &Arc<Sound>) {
        self.sender.send(Arc::clone(sound)).ok();
    }
}

impl Default for AudioDevice {
    fn default() -> Self {
        Self::new()
    }
}
