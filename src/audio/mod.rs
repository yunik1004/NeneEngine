use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc::{self, Sender},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
    meta::MetadataOptions, probe::Hint,
};

use crate::math::Vec2;

// ── Sound ─────────────────────────────────────────────────────────────────────

/// A decoded audio clip loaded into memory.
pub struct Sound {
    pub(crate) samples: Vec<f32>,
    pub(crate) channels: usize,
    pub(crate) sample_rate: u32,
}

impl Sound {
    /// Load an audio file from disk. Supports any format supported by Symphonia
    /// (mp3, ogg, flac, wav, …).
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
    pub fn sample_count(&self) -> usize {
        self.samples.len() / self.channels
    }
}

// ── PlayOptions ───────────────────────────────────────────────────────────────

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

// ── PlayHandle ────────────────────────────────────────────────────────────────

/// A handle to an in-progress playback. Dropping does **not** stop the sound.
pub struct PlayHandle {
    stopped: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    pan: Arc<AtomicU32>,
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

// ── AudioDevice (internal) ────────────────────────────────────────────────────

struct PlayRequest {
    sound: Arc<Sound>,
    volume: Arc<AtomicU32>,
    pan: Arc<AtomicU32>,
    looping: bool,
    stopped: Arc<AtomicBool>,
}

struct Playback {
    sound: Arc<Sound>,
    position: f64,
    volume: Arc<AtomicU32>,
    pan: Arc<AtomicU32>,
    looping: bool,
    stopped: Arc<AtomicBool>,
}

// ── AudioDevice ───────────────────────────────────────────────────────────────

/// The audio output device and mixer.
///
/// Keep alive for the entire application lifetime.
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
            .expect("Failed to build output stream");

        stream.play().expect("Failed to start audio stream");
        Self {
            _stream: stream,
            sender,
        }
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

impl Default for AudioDevice {
    fn default() -> Self {
        Self::new()
    }
}

fn mix_frame(out: &mut [f32], src: &[f32], volume: f32, pan: f32) {
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

// ── SpatialAudio ──────────────────────────────────────────────────────────────

/// Listener configuration for spatial (positional) audio.
///
/// Derive stereo pan and distance attenuation from 2-D world coordinates.
pub struct SpatialAudio {
    /// World position of the listener.
    pub listener: Vec2,
    /// Distance at which sounds become inaudible.
    pub max_distance: f32,
}

impl SpatialAudio {
    pub fn new(max_distance: f32) -> Self {
        Self {
            listener: Vec2::ZERO,
            max_distance: max_distance.max(f32::EPSILON),
        }
    }

    pub fn options_for(&self, emitter_pos: Vec2) -> PlayOptions {
        let (volume, pan) = self.compute(emitter_pos);
        PlayOptions {
            volume,
            pan,
            looping: false,
        }
    }

    pub fn play(&self, device: &AudioDevice, sound: &Arc<Sound>, emitter_pos: Vec2) -> PlayHandle {
        device.play_with(sound, self.options_for(emitter_pos))
    }

    pub fn play_source(
        &self,
        device: &AudioDevice,
        sound: &Arc<Sound>,
        emitter_pos: Vec2,
        looping: bool,
    ) -> SpatialSource {
        let opts = PlayOptions {
            looping,
            ..self.options_for(emitter_pos)
        };
        let handle = device.play_with(sound, opts);
        SpatialSource {
            handle,
            pos: emitter_pos,
        }
    }

    fn compute(&self, emitter_pos: Vec2) -> (f32, f32) {
        let dx = emitter_pos.x - self.listener.x;
        let dy = emitter_pos.y - self.listener.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let volume = (1.0 - dist / self.max_distance).clamp(0.0, 1.0);
        let pan = (dx / self.max_distance).clamp(-1.0, 1.0);
        (volume, pan)
    }
}

// ── SpatialSource ─────────────────────────────────────────────────────────────

/// A playing sound whose spatial parameters update as the emitter moves.
pub struct SpatialSource {
    handle: PlayHandle,
    pos: Vec2,
}

impl SpatialSource {
    pub fn set_position(&mut self, spatial: &SpatialAudio, pos: Vec2) {
        self.pos = pos;
        let (volume, pan) = spatial.compute(pos);
        self.handle.set_volume(volume);
        self.handle.set_pan(pan);
    }

    pub fn position(&self) -> Vec2 {
        self.pos
    }
    pub fn stop(&self) {
        self.handle.stop();
    }
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}
