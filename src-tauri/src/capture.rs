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

pub fn start(name: &str) -> Result<(cpal::Stream, Arc<Mutex<Audio>>), String> {
    let host = cpal::default_host();
    let device = if name.is_empty() {
        host.default_input_device()
    } else {
        host.input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().ok().as_deref() == Some(name))
    }
    .ok_or("Microphone unavailable. Reconnect it or choose another input.")?;
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
    Ok((stream, audio))
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
