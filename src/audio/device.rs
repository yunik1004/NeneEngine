use super::Sound;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc::{self, Sender},
};

// ── Public types ──────────────────────────────────────────────────────────────

/// Options for a single playback instance.
///
/// Build with `PlayOptions::default()` and modify fields as needed:
/// ```
/// use nene::audio::PlayOptions;
/// let opts = PlayOptions { volume: 0.5, pan: -0.8, looping: true };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PlayOptions {
    /// Amplitude multiplier. `1.0` = full volume, `0.0` = silent. Clamped to `[0.0, 1.0]`.
    pub volume: f32,
    /// Stereo position. `-1.0` = hard left, `0.0` = centre, `1.0` = hard right.
    /// Has no effect on mono output devices. Clamped to `[-1.0, 1.0]`.
    pub pan: f32,
    /// Whether the sound restarts when it reaches the end.
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

/// A handle to an in-progress playback returned by [`AudioDevice::play`] /
/// [`AudioDevice::play_with`].
///
/// Dropping the handle does **not** stop playback — call [`stop`](Self::stop)
/// explicitly if you need early termination.
pub struct PlayHandle {
    stopped: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    pan: Arc<AtomicU32>,
}

impl PlayHandle {
    /// Stop this playback on the next audio callback. Idempotent.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }

    /// `true` once the sound has finished playing (or [`stop`](Self::stop) was called).
    pub fn is_finished(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    /// Update the volume of this playback in real time. Clamped to `[0.0, 1.0]`.
    pub fn set_volume(&self, volume: f32) {
        self.volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    /// Update the stereo pan of this playback in real time. Clamped to `[-1.0, 1.0]`.
    pub fn set_pan(&self, pan: f32) {
        self.pan
            .store(pan.clamp(-1.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}

// ── Internal ──────────────────────────────────────────────────────────────────

struct PlayRequest {
    sound: Arc<Sound>,
    volume: Arc<AtomicU32>,
    pan: Arc<AtomicU32>,
    looping: bool,
    stopped: Arc<AtomicBool>,
}

struct Playback {
    sound: Arc<Sound>,
    /// Fractional frame position in the source audio.
    position: f64,
    volume: Arc<AtomicU32>,
    pan: Arc<AtomicU32>,
    looping: bool,
    stopped: Arc<AtomicBool>,
}

// ── AudioDevice ───────────────────────────────────────────────────────────────

/// The audio output device and mixer.
///
/// Create once at startup and keep alive for the duration of the application.
/// Sounds submitted via [`play`](Self::play) or [`play_with`](Self::play_with)
/// are mixed in real-time on a background thread.
pub struct AudioDevice {
    _stream: cpal::Stream,
    sender: Sender<PlayRequest>,
}

impl AudioDevice {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device found");
        let config = device.default_output_config().expect("No output config");

        let sample_rate = config.sample_rate();
        let out_channels = config.channels() as usize;

        let (sender, receiver) = mpsc::channel::<PlayRequest>();
        let mut playbacks: Vec<Playback> = Vec::new();

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    // Receive new sounds.
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
                            // Advance past the end: loop or stop.
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
            .expect("Failed to build output stream");

        stream.play().expect("Failed to start audio stream");

        Self {
            _stream: stream,
            sender,
        }
    }

    /// Play `sound` at full volume, centred, without looping.
    ///
    /// Returns a [`PlayHandle`] that can be used to stop playback early or
    /// check when it finishes. Dropping the handle does **not** stop the sound.
    pub fn play(&self, sound: &Arc<Sound>) -> PlayHandle {
        self.play_with(sound, PlayOptions::default())
    }

    /// Play `sound` with explicit [`PlayOptions`].
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

impl Default for AudioDevice {
    fn default() -> Self {
        Self::new()
    }
}

// ── Mixing helper ─────────────────────────────────────────────────────────────

/// Mix one audio frame from `src` into `out`, applying `volume` and `pan`.
///
/// Linear pan law: `left_gain  = (1 − pan) / 2 × volume`
///                 `right_gain = (1 + pan) / 2 × volume`
fn mix_frame(out: &mut [f32], src: &[f32], volume: f32, pan: f32) {
    let src_ch = src.len();
    match out.len() {
        1 => {
            let sample = src.iter().sum::<f32>() / src_ch as f32;
            out[0] += sample * volume;
        }
        2 => {
            let left_gain = (1.0 - pan) * 0.5 * volume;
            let right_gain = (1.0 + pan) * 0.5 * volume;
            let l = src[0];
            let r = if src_ch >= 2 { src[1] } else { src[0] };
            out[0] += l * left_gain;
            out[1] += r * right_gain;
        }
        n => {
            for (ch, o) in out.iter_mut().enumerate() {
                let src_ch_idx = ch.min(src_ch - 1);
                *o += src[src_ch_idx] * volume;
            }
            let _ = n;
        }
    }
}
