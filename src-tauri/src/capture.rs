use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

pub const MAX_SECONDS: usize = 120;

pub fn devices() -> Result<Vec<String>, String> {
    cpal::default_host()
        .input_devices()
        .map_err(|e| e.to_string())?
        .map(|d| d.name().map_err(|e| e.to_string()))
        .collect()
}

pub struct Audio {
    pub samples: Vec<i16>,
    pub rate: u32,
    pub error: Option<String>,
}

fn select_input<T>(
    preferred: &str,
    mut inputs: impl Iterator<Item = (String, T)>,
    default: impl FnOnce() -> Option<T>,
) -> Option<T> {
    if !preferred.is_empty() {
        if let Some((_, input)) = inputs.find(|(name, _)| name == preferred) {
            return Some(input);
        }
    }
    default()
}

pub fn start(name: &str) -> Result<(cpal::Stream, Arc<Mutex<Audio>>, String), String> {
    let host = cpal::default_host();
    // Re-enumerate for each explicit recording. Keep the saved preference so a
    // reconnected headset is used again; do not pin the temporary fallback.
    let inputs = host
        .input_devices()
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|device| device.name().ok().map(|name| (name, device)));
    let device = select_input(name, inputs, || host.default_input_device())
        .ok_or("No microphone available. Connect one or check Windows input settings.")?;
    let input_name = device.name().unwrap_or_else(|_| "Windows input".into());
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let rate = supported.sample_rate().0;
    let config: cpal::StreamConfig = supported.clone().into();
    let audio = Arc::new(Mutex::new(Audio {
        samples: Vec::with_capacity(rate as usize * MAX_SECONDS),
        rate,
        error: None,
    }));
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => stream::<f32>(&device, &config, audio.clone()),
        cpal::SampleFormat::I16 => stream::<i16>(&device, &config, audio.clone()),
        cpal::SampleFormat::U16 => stream::<u16>(&device, &config, audio.clone()),
        other => {
            return Err(format!(
                "Microphone format {other:?} is unsupported. Choose another input."
            ))
        }
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, audio, input_name))
}

fn stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    audio: Arc<Mutex<Audio>>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let error_audio = audio.clone();
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            let mut audio = audio.lock().unwrap();
            let limit = audio.rate as usize * MAX_SECONDS;
            for frame in data.chunks_exact(channels) {
                if audio.samples.len() >= limit {
                    break;
                }
                let value = frame
                    .iter()
                    .map(|sample| sample.to_sample::<f32>())
                    .sum::<f32>()
                    / channels as f32;
                audio
                    .samples
                    .push((value.clamp(-1., 1.) * i16::MAX as f32) as i16);
            }
        },
        move |error| {
            error_audio.lock().unwrap().error =
                Some(format!("Microphone disconnected or failed: {error}"));
        },
        Some(Duration::from_secs(3)),
    )
}

pub fn save(path: &Path, rate: u32, samples: &[i16]) -> Result<(), String> {
    if samples.is_empty() {
        return Err("No audio was captured. Hold the shortcut a little longer.".into());
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for sample in samples {
        writer.write_sample(*sample).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferred_headset_falls_back_and_is_restored_after_reconnection() {
        let connected = || vec![("Laptop".into(), 1), ("Headset".into(), 2)].into_iter();
        assert_eq!(
            select_input("Headset", connected(), || panic!(
                "Do not use the default when the headset is present"
            )),
            Some(2)
        );
        assert_eq!(
            select_input("Headset", vec![("Laptop".into(), 1)].into_iter(), || Some(
                1
            )),
            Some(1)
        );
        assert_eq!(select_input("Headset", connected(), || Some(1)), Some(2));
    }

    #[test]
    fn automatic_input_resolves_the_current_default_each_time() {
        assert_eq!(
            select_input("", std::iter::empty::<(String, i32)>(), || Some(1)),
            Some(1)
        );
        assert_eq!(
            select_input("", std::iter::empty::<(String, i32)>(), || Some(2)),
            Some(2)
        );
        assert_eq!(
            select_input("Missing", std::iter::empty::<(String, i32)>(), || None),
            None
        );
    }

    #[test]
    fn wav_has_expected_rate_and_frames() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("capture.wav");
        save(&path, 48000, &[0; 4800]).unwrap();
        let reader = hound::WavReader::open(path).unwrap();
        assert_eq!(reader.spec().sample_rate, 48000);
        assert_eq!(reader.duration(), 4800);
        assert!(save(&directory.path().join("empty.wav"), 16000, &[]).is_err());
    }
}
