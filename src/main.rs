use signal::*;

#[cfg(feature = "audio")]
mod audio {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{Device, StreamConfig};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

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

        pub fn play_signal(
            &self,
            signal: &signal::Signal,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let samples = signal.samples.clone();
            let sample_rate = self.config.sample_rate.0 as f32;
            let channels = self.config.channels as usize;

            let samples = Arc::new(Mutex::new(samples));
            let sample_index = Arc::new(Mutex::new(0usize));

            let samples_clone = Arc::clone(&samples);
            let sample_index_clone = Arc::clone(&sample_index);

            let stream = self.device.build_output_stream(
                &self.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let samples_lock = samples_clone.lock().unwrap();
                    let mut index_lock = sample_index_clone.lock().unwrap();

                    for frame in data.chunks_mut(channels) {
                        let sample = if *index_lock < samples_lock.len() {
                            samples_lock[*index_lock]
                        } else {
                            0.0
                        };

                        for channel_sample in frame.iter_mut() {
                            *channel_sample = sample;
                        }

                        *index_lock += 1;
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?;

            stream.play()?;

            let duration = samples.lock().unwrap().len() as f32 / sample_rate;
            thread::sleep(Duration::from_secs_f32(duration + 0.5));

            Ok(())
        }
    }

    pub fn play_signal(signal: &signal::Signal) -> Result<(), Box<dyn std::error::Error>> {
        let player = AudioPlayer::new()?;
        player.play_signal(signal)
    }

    pub fn save_wav(
        signal: &signal::Signal,
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: signal.sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(filename, spec)?;

        for &sample in &signal.samples {
            writer.write_sample(sample)?;
        }

        writer.finalize()?;
        Ok(())
    }
}

fn main() {
    println!("Generating audio...");

    let mut s = Signal::new(44100);
    generate_audio(&mut s);

    println!("Generated {} samples", s.len());

    #[cfg(feature = "audio")]
    {
        println!("Playing audio...");
        if let Err(e) = audio::play_signal(&s) {
            eprintln!("Error playing audio: {}", e);
        } else {
            println!("Audio playback completed!");
        }

        println!("Saving to output.wav...");
        if let Err(e) = audio::save_wav(&s, "output.wav") {
            eprintln!("Error saving wav: {}", e);
        } else {
            println!("Saved to output.wav");
        }
    }

    #[cfg(not(feature = "audio"))]
    {
        println!("Audio features not enabled. Build with --features audio to play/save audio.");
        println!("Sample data: {:?}", &s.samples[0..10.min(s.samples.len())]);
    }
}

fn generate_audio(s: &mut Signal) {
    let seq = sequence([chord(&[0, 2, 4]), chord(&[4, 6, 8]), chord(&[8, 10, 12])]);

    let env = adsr(
        (vol(0.8), time(0.1)),
        (vol(0.6), time(0.1)),
        (vol(0.4), time(0.8)),
        (vol(0.0), time(0.5)),
    );

    for _ in 0..44100 * 3 {
        let pitches = seq.output(s);
        let envelope_value = env.output();

        for pitch in pitches {
            sin()
                .phase(0.0)
                .pitch(pitch)
                .atten(envelope_value * 0.2)
                .play(s);
        }

        s.advance();
    }
}
