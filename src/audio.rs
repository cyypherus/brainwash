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

    pub fn play_live<F>(&self, synth_fn: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&mut Signal) + Send + 'static,
    {
        let signal = Arc::new(Mutex::new(Signal::new(self.config.sample_rate.0 as usize)));
        let channels = self.config.channels as usize;

        let synth_fn = Arc::new(Mutex::new(synth_fn));

        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut signal_lock = signal.lock().unwrap();
                let mut synth_lock = synth_fn.lock().unwrap();

                for frame in data.chunks_mut(channels) {
                    synth_lock(&mut *signal_lock);

                    let sample = signal_lock.get_current_sample();

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

pub fn play_live<F>(synth_fn: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(&mut Signal) + Send + 'static,
{
    let player = AudioPlayer::new()?;
    player.play_live(synth_fn)
}

pub fn save_wav<F>(
    mut synth_fn: F,
    filename: &str,
    duration_seconds: f32,
    sample_rate: usize,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(&mut Signal),
{
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(filename, spec)?;
    let mut signal = Signal::new(sample_rate);
    let total_samples = (duration_seconds * sample_rate as f32) as usize;

    for _ in 0..total_samples {
        synth_fn(&mut signal);
        writer.write_sample(signal.get_current_sample())?;
        signal.advance();
    }

    writer.finalize()?;
    Ok(())
}
