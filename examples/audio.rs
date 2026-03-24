use nene::audio::{AudioDevice, Sound};
use std::sync::Arc;

fn main() {
    let device = AudioDevice::new();
    let sample_rate = 44100u32;

    // 모노: 440Hz 사인파
    let mono_samples: Vec<i16> = (0..sample_rate)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            ((t * 440.0 * 2.0 * std::f32::consts::PI).sin() * i16::MAX as f32) as i16
        })
        .collect();

    let mono_path = std::env::temp_dir().join("nene_mono.wav");
    std::fs::write(&mono_path, make_wav(sample_rate, 1, &mono_samples)).unwrap();
    let mono = Arc::new(Sound::load(&mono_path));
    println!(
        "Mono:   {}Hz, {} ch, {} frames",
        mono.sample_rate(),
        mono.channels(),
        mono.sample_count()
    );

    // 스테레오: 좌 440Hz / 우 880Hz
    let stereo_samples: Vec<i16> = (0..sample_rate)
        .flat_map(|i| {
            let t = i as f32 / sample_rate as f32;
            let left = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
            let right = (t * 880.0 * 2.0 * std::f32::consts::PI).sin();
            [
                (left * i16::MAX as f32) as i16,
                (right * i16::MAX as f32) as i16,
            ]
        })
        .collect();

    let stereo_path = std::env::temp_dir().join("nene_stereo.wav");
    std::fs::write(&stereo_path, make_wav(sample_rate, 2, &stereo_samples)).unwrap();
    let stereo = Arc::new(Sound::load(&stereo_path));
    println!(
        "Stereo: {}Hz, {} ch, {} frames",
        stereo.sample_rate(),
        stereo.channels(),
        stereo.sample_count()
    );

    println!("\nPlaying mono 440Hz...");
    device.play(&mono);
    std::thread::sleep(std::time::Duration::from_secs(1));

    println!("Playing stereo (left 440Hz / right 880Hz)...");
    device.play(&stereo);
    std::thread::sleep(std::time::Duration::from_secs(1));

    println!("Playing mono + stereo simultaneously...");
    device.play(&mono);
    device.play(&stereo);
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("Done.");
}

fn make_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let mut buf = Vec::with_capacity(44 + samples.len() * 2);

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size).to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
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
