use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc::{self, Sender},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::sound::Sound;

/// Options for a single playback instance.
#[derive(Debug, Clone, Copy)]
pub struct PlayOptions {
    /// Amplitude multiplier `[0.0, 1.0]`.
    pub volume: f32,
    /// Stereo position `[-1.0, 1.0]`.
    pub pan: f32,
    pub looping: bool,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pan: 0.0,
            looping: false,
        }
    }
}

/// A handle to an in-progress playback. Dropping does **not** stop the sound.
pub struct PlayHandle {
    pub(super) stopped: Arc<AtomicBool>,
    pub(super) volume: Arc<AtomicU32>,
    pub(super) pan: Arc<AtomicU32>,
}

impl PlayHandle {
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
    pub fn is_finished(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }
    pub fn set_volume(&self, volume: f32) {
        self.volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
    pub fn set_pan(&self, pan: f32) {
        self.pan
            .store(pan.clamp(-1.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}

pub(super) struct PlayRequest {
    pub sound: Arc<Sound>,
    pub volume: Arc<AtomicU32>,
    pub pan: Arc<AtomicU32>,
    pub looping: bool,
    pub stopped: Arc<AtomicBool>,
}

struct Playback {
    sound: Arc<Sound>,
    position: f64,
    volume: Arc<AtomicU32>,
    pan: Arc<AtomicU32>,
    looping: bool,
    stopped: Arc<AtomicBool>,
}

/// The audio output device and mixer.
///
/// Keep alive for the entire application lifetime.
pub struct AudioDevice {
    _stream: cpal::Stream,
    sender: Sender<PlayRequest>,
}

impl AudioDevice {
    /// Create the audio output device and mixer.
    ///
    /// Returns `None` if no output device is available (headless servers, WSL,
    /// some CI environments). Game logic should check for `None` and skip audio.
    pub fn new() -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;

        let sample_rate = config.sample_rate();
        let out_channels = config.channels() as usize;

        let (sender, receiver) = mpsc::channel::<PlayRequest>();
        let mut playbacks: Vec<Playback> = Vec::new();

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    while let Ok(req) = receiver.try_recv() {
                        playbacks.push(Playback {
                            sound: req.sound,
                            position: 0.0,
                            volume: req.volume,
                            pan: req.pan,
                            looping: req.looping,
                            stopped: req.stopped,
                        });
                    }
                    for s in data.iter_mut() {
                        *s = 0.0;
                    }
                    playbacks.retain_mut(|pb| {
                        if pb.stopped.load(Ordering::Relaxed) {
                            return false;
                        }
                        let ratio = pb.sound.sample_rate as f64 / sample_rate as f64;
                        let src_ch = pb.sound.channels;
                        let total_frames = pb.sound.samples.len() / src_ch;
                        for frame in data.chunks_mut(out_channels) {
                            if pb.position as usize >= total_frames {
                                if pb.looping {
                                    pb.position -= total_frames as f64;
                                    if pb.position < 0.0 {
                                        pb.position = 0.0;
                                    }
                                } else {
                                    pb.stopped.store(true, Ordering::Relaxed);
                                    return false;
                                }
                            }
                            let src_idx = pb.position as usize * src_ch;
                            let vol = f32::from_bits(pb.volume.load(Ordering::Relaxed));
                            let pan = f32::from_bits(pb.pan.load(Ordering::Relaxed));
                            mix_frame(
                                frame,
                                &pb.sound.samples[src_idx..src_idx + src_ch],
                                vol,
                                pan,
                            );
                            pb.position += ratio;
                        }
                        true
                    });
                },
                |err| eprintln!("Audio stream error: {err}"),
                None,
            )
            .ok()?;

        stream.play().ok()?;
        Some(Self {
            _stream: stream,
            sender,
        })
    }

    /// Play at full volume, centred, without looping.
    pub fn play(&self, sound: &Arc<Sound>) -> PlayHandle {
        self.play_with(sound, PlayOptions::default())
    }

    pub fn play_with(&self, sound: &Arc<Sound>, options: PlayOptions) -> PlayHandle {
        let stopped = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(AtomicU32::new(options.volume.clamp(0.0, 1.0).to_bits()));
        let pan = Arc::new(AtomicU32::new(options.pan.clamp(-1.0, 1.0).to_bits()));
        let handle = PlayHandle {
            stopped: Arc::clone(&stopped),
            volume: Arc::clone(&volume),
            pan: Arc::clone(&pan),
        };
        self.sender
            .send(PlayRequest {
                sound: Arc::clone(sound),
                volume,
                pan,
                looping: options.looping,
                stopped,
            })
            .ok();
        handle
    }
}

pub(super) fn mix_frame(out: &mut [f32], src: &[f32], volume: f32, pan: f32) {
    let src_ch = src.len();
    match out.len() {
        1 => {
            out[0] += src.iter().sum::<f32>() / src_ch as f32 * volume;
        }
        2 => {
            let l = src[0];
            let r = if src_ch >= 2 { src[1] } else { src[0] };
            out[0] += l * (1.0 - pan) * 0.5 * volume;
            out[1] += r * (1.0 + pan) * 0.5 * volume;
        }
        _ => {
            for (ch, o) in out.iter_mut().enumerate() {
                *o += src[ch.min(src_ch - 1)] * volume;
            }
        }
    }
}
