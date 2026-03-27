use nene::audio::{AudioDevice, PlayOptions, Sound};
use std::sync::Arc;

/// 테스트용 최소 WAV 파일을 메모리에서 생성한다.
fn make_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(44 + samples.len() * 2);

    // RIFF 헤더
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt 청크
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // 청크 크기
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&(channels * 2).to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data 청크
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    buf
}

fn write_temp_wav(
    name: &str,
    sample_rate: u32,
    channels: u16,
    samples: &[i16],
) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, make_wav(sample_rate, channels, samples)).unwrap();
    path
}

#[test]
fn sound_load_mono() {
    let samples: Vec<i16> = (0..44100)
        .map(|i| ((i as f32 * 0.1).sin() * 1000.0) as i16)
        .collect();
    let path = write_temp_wav("nene_test_mono.wav", 44100, 1, &samples);

    let sound = Sound::load(&path);
    assert_eq!(sound.channels(), 1);
    assert_eq!(sound.sample_rate(), 44100);
    assert_eq!(sound.sample_count(), samples.len());
}

#[test]
fn sound_load_stereo() {
    let samples: Vec<i16> = vec![0i16; 44100 * 2];
    let path = write_temp_wav("nene_test_stereo.wav", 44100, 2, &samples);

    let sound = Sound::load(&path);
    assert_eq!(sound.channels(), 2);
    assert_eq!(sound.sample_rate(), 44100);
    assert_eq!(sound.sample_count(), samples.len() / 2); // 프레임 기준
}

#[test]
fn sound_load_different_sample_rate() {
    let samples: Vec<i16> = vec![0i16; 22050];
    let path = write_temp_wav("nene_test_22050.wav", 22050, 1, &samples);

    let sound = Sound::load(&path);
    assert_eq!(sound.sample_rate(), 22050);
}

#[test]
fn audio_device_creation() {
    let _ = AudioDevice::new();
}

#[test]
fn play_does_not_panic() {
    let samples: Vec<i16> = vec![0i16; 4410];
    let path = write_temp_wav("nene_test_play.wav", 44100, 1, &samples);

    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    device.play(&sound);
}

#[test]
fn play_same_sound_simultaneously() {
    let samples: Vec<i16> = vec![0i16; 4410];
    let path = write_temp_wav("nene_test_simultaneous.wav", 44100, 1, &samples);

    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    device.play(&sound);
    device.play(&sound);
    device.play(&sound);
}

#[test]
fn play_stereo() {
    let samples: Vec<i16> = (0..4410)
        .flat_map(|i| {
            let t = i as f32 / 44100.0;
            let left = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
            let right = (t * 880.0 * 2.0 * std::f32::consts::PI).sin();
            [
                (left * i16::MAX as f32) as i16,
                (right * i16::MAX as f32) as i16,
            ]
        })
        .collect();
    let path = write_temp_wav("nene_test_stereo_play.wav", 44100, 2, &samples);

    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    assert_eq!(sound.channels(), 2);
    device.play(&sound);
}

// ── PlayOptions & PlayHandle ──────────────────────────────────────────────────

#[test]
fn play_with_volume_does_not_panic() {
    let samples: Vec<i16> = vec![0i16; 4410];
    let path = write_temp_wav("nene_test_volume.wav", 44100, 1, &samples);
    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    device.play_with(&sound, PlayOptions { volume: 0.5, ..Default::default() });
}

#[test]
fn play_with_pan_left_does_not_panic() {
    let samples: Vec<i16> = vec![0i16; 4410];
    let path = write_temp_wav("nene_test_pan_l.wav", 44100, 1, &samples);
    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    device.play_with(&sound, PlayOptions { pan: -1.0, ..Default::default() });
}

#[test]
fn play_with_pan_right_does_not_panic() {
    let samples: Vec<i16> = vec![0i16; 4410];
    let path = write_temp_wav("nene_test_pan_r.wav", 44100, 1, &samples);
    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    device.play_with(&sound, PlayOptions { pan: 1.0, ..Default::default() });
}

#[test]
fn play_with_looping_does_not_panic() {
    let samples: Vec<i16> = vec![0i16; 4410];
    let path = write_temp_wav("nene_test_loop.wav", 44100, 1, &samples);
    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    let handle = device.play_with(&sound, PlayOptions { looping: true, ..Default::default() });
    // Give it a moment, then stop explicitly to avoid playing forever in tests.
    std::thread::sleep(std::time::Duration::from_millis(20));
    handle.stop();
}

#[test]
fn play_handle_stop_signals_finished() {
    let samples: Vec<i16> = vec![0i16; 44100]; // 1 second
    let path = write_temp_wav("nene_test_stop.wav", 44100, 1, &samples);
    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    let handle = device.play(&sound);
    assert!(!handle.is_finished(), "should not be finished immediately after starting");
    handle.stop();
    assert!(handle.is_finished(), "should be finished after stop()");
}

#[test]
fn play_handle_stop_is_idempotent() {
    let samples: Vec<i16> = vec![0i16; 44100];
    let path = write_temp_wav("nene_test_stop2.wav", 44100, 1, &samples);
    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    let handle = device.play(&sound);
    handle.stop();
    handle.stop(); // must not panic
}

#[test]
fn volume_out_of_range_is_clamped() {
    // volume=2.0 should not panic (clamped to 1.0)
    let samples: Vec<i16> = vec![0i16; 4410];
    let path = write_temp_wav("nene_test_vol_clamp.wav", 44100, 1, &samples);
    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    device.play_with(&sound, PlayOptions { volume: 2.0, ..Default::default() });
}

#[test]
fn pan_out_of_range_is_clamped() {
    let samples: Vec<i16> = vec![0i16; 4410];
    let path = write_temp_wav("nene_test_pan_clamp.wav", 44100, 1, &samples);
    let device = AudioDevice::new();
    let sound = Arc::new(Sound::load(&path));
    device.play_with(&sound, PlayOptions { pan: 99.0, ..Default::default() });
}

#[test]
fn play_mono_and_stereo_simultaneously() {
    let mono_samples: Vec<i16> = vec![0i16; 4410];
    let mono_path = write_temp_wav("nene_test_mix_mono.wav", 44100, 1, &mono_samples);

    let stereo_samples: Vec<i16> = vec![0i16; 4410 * 2];
    let stereo_path = write_temp_wav("nene_test_mix_stereo.wav", 44100, 2, &stereo_samples);

    let device = AudioDevice::new();
    let mono = Arc::new(Sound::load(&mono_path));
    let stereo = Arc::new(Sound::load(&stereo_path));

    device.play(&mono);
    device.play(&stereo);
}
