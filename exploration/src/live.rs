use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig};

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
        mut synth: impl FnMut(usize) -> f32 + Send + 'static,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let channels = self.config.channels as usize;

        let mut counter = 0;
        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    #[cfg(all(debug_assertions, feature = "no-alloc"))]
                    let sample = assert_no_alloc(|| synth(counter).clamp(-1., 1.));
                    #[cfg(not(all(debug_assertions, feature = "no-alloc")))]
                    let sample = synth(counter).clamp(-1., 1.);

                    for channel_sample in frame.iter_mut() {
                        *channel_sample = sample;
                    }
                    counter += 1;
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
    synth: impl FnMut(usize) -> f32 + Send + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let player = AudioPlayer::new()?;
    player.play_live(synth)
}
