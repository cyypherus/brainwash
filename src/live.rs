use crate::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig};
use std::sync::{Arc, Mutex};

use assert_no_alloc::*;

#[cfg(debug_assertions)] // required when disable_release is set (default)
#[global_allocator]
static A: AllocDisabler = AllocDisabler;

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

    pub fn play_live(
        &self,
        synth: impl FnMut(&mut Signal) -> f32 + Send + 'static,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let signal = Arc::new(Mutex::new(Signal::new(self.config.sample_rate.0 as usize)));
        let channels = self.config.channels as usize;

        let synth = Arc::new(Mutex::new(synth));

        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut signal_lock = signal.lock().unwrap();
                let mut synth = synth.lock().unwrap();

                for frame in data.chunks_mut(channels) {
                    let sample = assert_no_alloc(|| synth(&mut signal_lock).clamp(-1., 1.));

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

pub fn play_live(
    synth: impl FnMut(&mut Signal) -> f32 + Send + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let player = AudioPlayer::new()?;
    player.play_live(synth)
}
