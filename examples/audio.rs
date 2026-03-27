//! Audio demo — volume, pan, looping, and stop.
//!
//! Generates sine-wave WAV files in the system temp directory and plays them
//! through the default output device, demonstrating:
//!   - `play()` — fire-and-forget at full volume
//!   - `play_with(options)` — volume, stereo pan, looping
//!   - `PlayHandle::stop()` — early stop on a looping sound

use nene::audio::{AudioDevice, PlayOptions, Sound};
use std::{sync::Arc, time::Duration};

fn main() {
    let device = AudioDevice::new();
    let sr = 44100u32;

    // ── Build a couple of test tones ──────────────────────────────────────────
    let a4 = Arc::new(sine_wav(sr, 1, 440.0, 1.0)); // 1 s mono A4
    let a5 = Arc::new(sine_wav(sr, 2, 880.0, 1.0)); // 1 s stereo A5
    let blip = Arc::new(sine_wav(sr, 1, 660.0, 0.1)); // 0.1 s mono blip

    println!(
        "A4  mono:   {}Hz, {}ch, {} frames",
        a4.sample_rate(),
        a4.channels(),
        a4.sample_count()
    );
    println!(
        "A5  stereo: {}Hz, {}ch, {} frames",
        a5.sample_rate(),
        a5.channels(),
        a5.sample_count()
    );
    println!(
        "Blip mono:  {}Hz, {}ch, {} frames",
        blip.sample_rate(),
        blip.channels(),
        blip.sample_count()
    );

    // ── 1. Plain play ─────────────────────────────────────────────────────────
    println!("\n[1] A4 at full volume...");
    let h = device.play(&a4);
    sleep(1100);
    println!("    finished: {}", h.is_finished());

    // ── 2. Half volume ────────────────────────────────────────────────────────
    println!("\n[2] A4 at 30% volume...");
    device.play_with(
        &a4,
        PlayOptions {
            volume: 0.30,
            ..Default::default()
        },
    );
    sleep(1100);

    // ── 3. Pan hard left / hard right ─────────────────────────────────────────
    println!("\n[3] A4 panned hard LEFT...");
    device.play_with(
        &a4,
        PlayOptions {
            pan: -1.0,
            ..Default::default()
        },
    );
    sleep(600);
    println!("    A4 panned hard RIGHT...");
    device.play_with(
        &a4,
        PlayOptions {
            pan: 1.0,
            ..Default::default()
        },
    );
    sleep(1100);

    // ── 4. Simultaneous: left A4 + right A5 ──────────────────────────────────
    println!("\n[4] A4 hard-left and A5 hard-right simultaneously...");
    device.play_with(
        &a4,
        PlayOptions {
            pan: -1.0,
            ..Default::default()
        },
    );
    device.play_with(
        &a5,
        PlayOptions {
            pan: 1.0,
            ..Default::default()
        },
    );
    sleep(1200);

    // ── 5. Looping blip, stopped after 500 ms ────────────────────────────────
    println!("\n[5] Blip looping — stops in 500 ms...");
    let looping = device.play_with(
        &blip,
        PlayOptions {
            looping: true,
            volume: 0.7,
            ..Default::default()
        },
    );
    sleep(500);
    looping.stop();
    println!("    stopped. is_finished={}", looping.is_finished());
    sleep(200); // brief silence to confirm it stopped

    println!("\nDone.");
}

fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// Generate a sine-wave WAV at `freq` Hz for `duration` seconds.
fn sine_wav(sample_rate: u32, channels: u16, freq: f32, duration: f32) -> Sound {
    let n_frames = (sample_rate as f32 * duration) as usize;
    let samples: Vec<i16> = (0..n_frames)
        .flat_map(|i| {
            let t = i as f32 / sample_rate as f32;
            let v = (t * freq * std::f32::consts::TAU).sin();
            let s = (v * i16::MAX as f32) as i16;
            // Replicate the same sample across all channels.
            vec![s; channels as usize]
        })
        .collect();

    let data = make_wav(sample_rate, channels, &samples);
    let path = std::env::temp_dir().join(format!("nene_audio_{freq:.0}hz_{channels}ch.wav"));
    std::fs::write(&path, data).unwrap();
    Sound::load(&path)
}

fn make_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let mut buf = Vec::with_capacity(44 + samples.len() * 2);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
    buf.extend_from_slice(&(channels * 2).to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}
