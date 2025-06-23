use crate::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig};
use std::sync::{Arc, Mutex};

pub struct AudioPlayer {
    device: Device,
    config: StreamConfig,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let config = device.default_output_config()?.into();

        Ok(AudioPlayer { device, config })
    }

    pub fn play_live(&self, synth: impl Synth) -> Result<(), Box<dyn std::error::Error>> {
        let signal = Arc::new(Mutex::new(Signal::new(self.config.sample_rate.0 as usize)));
        let channels = self.config.channels as usize;

        let synth = Arc::new(Mutex::new(synth));

        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut signal_lock = signal.lock().unwrap();
                let mut synth = synth.lock().unwrap();

                for frame in data.chunks_mut(channels) {
                    let sample = synth.limited(&mut signal_lock);

                    for channel_sample in frame.iter_mut() {
                        *channel_sample = sample;
                    }

                    signal_lock.advance();
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;

        println!("Playing live audio... Press Ctrl+C to stop");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

pub fn play_live(synth: impl Synth) -> Result<(), Box<dyn std::error::Error>> {
    let player = AudioPlayer::new()?;
    player.play_live(synth)
}

pub fn save_wav(
    mut synth: impl Synth,
    filename: &str,
    duration_seconds: f32,
    sample_rate: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(filename, spec)?;
    let mut signal = Signal::new(sample_rate);
    let total_samples = (duration_seconds * sample_rate as f32) as usize;

    for _ in 0..total_samples {
        let sample = synth.limited(&mut signal);
        writer.write_sample(sample)?; // Left channel
        writer.write_sample(sample)?; // Right channel
        signal.advance();
    }

    writer.finalize()?;
    Ok(())
}
