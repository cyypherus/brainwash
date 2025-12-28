use std::array;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PressState {
    Idle,
    Pressed {
        pressed_at: usize,
        time_in_state: usize,
    },
    Released {
        pressed_at: usize,
        released_at: usize,
        time_in_state: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub pitch: u8,
    pub frequency: f32,
}

pub struct Instrument<V> {
    voices: [V; 128],
    cached_pressed_pitches: Option<Vec<u8>>,
    cached_press_state: Option<PressState>,
}

impl<V> Instrument<V>
where
    V: FnMut(Key, usize) -> f32 + Send,
{
    pub fn new(voice_builder: impl Fn() -> V + Send + 'static) -> Self {
        Instrument {
            voices: array::from_fn(|_| voice_builder()),
            cached_pressed_pitches: None,
            cached_press_state: None,
        }
    }

    pub fn set_pressed_pitches(&mut self, pitches: Vec<u8>, press_state: PressState) {
        self.cached_pressed_pitches = Some(pitches);
        self.cached_press_state = Some(press_state);
    }

    pub fn output(&mut self, current_sample: usize) -> f32 {
        let mut sum = 0.0f32;
        for (pitch, voice) in self.voices.iter_mut().enumerate() {
            let key = Key {
                pitch: pitch as u8,
                frequency: 440.0 * 2.0_f32.powf((pitch as f32 - 69.0) / 12.0),
            };
            sum += voice(key, current_sample);
        }
        sum
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::chromatic;

//     use super::*;

//     #[test]
//     fn test_simple_note_sequence_presses_and_releases() {
//         let track_str = "(0/1/2)";
//         let track = crate::track::parse(track_str).expect("Failed to parse track");

//         let mut instrument = Instrument::new(crate::chromatic(), || {
//             let mut voice = crate::sin().build();
//             move |key| voice.output(key)
//         });

//         instrument.load_track(track, 120.0);

//         // At sample 0, note 0 should press
//         instrument.step_track(0);
//         assert!(
//             !matches!(instrument.keys[0].press_state, PressState::Idle),
//             "Note 0 should be pressed at sample 0"
//         );

//         // Advance to end of first division
//         let bar_duration = (44100.0 * 60.0 / 120.0 * 4.0) as usize;
//         let div_duration = bar_duration / 3;

//         instrument.step_track(div_duration);
//         // Note 0 should be released and note 1 pressed
//         assert!(
//             !matches!(instrument.keys[1].press_state, PressState::Idle),
//             "Note 1 should be pressed"
//         );
//     }

//     #[test]
//     fn test_polyphonic_layers_press_simultaneously() {
//         let track_str = "{(0)%(1)}";
//         let track = crate::track::parse(track_str).expect("Failed to parse track");

//         let mut instrument = Instrument::new(chromatic(), || {
//             let mut voice = crate::sin().build();
//             move |key| voice.output(key)
//         });

//         instrument.load_track(track, 120.0);

//         // At sample 0, both notes 0 and 1 should press
//         instrument.step_track(0);
//         assert!(
//             !matches!(instrument.keys[0].press_state, PressState::Idle),
//             "Note 0 should be pressed"
//         );
//         assert!(
//             !matches!(instrument.keys[1].press_state, PressState::Idle),
//             "Note 1 should be pressed"
//         );
//     }

//     #[test]
//     fn test_rest_does_not_create_voice() {
//         let track_str = "(0/_/1)";
//         let track = crate::track::parse(track_str).expect("Failed to parse track");

//         let mut instrument = Instrument::new(chromatic(), || {
//             let mut voice = crate::sin().build();
//             move |key| voice.output(key)
//         });

//         instrument.load_track(track, 120.0);

//         let bar_duration = (44100.0 * 60.0 / 120.0 * 4.0) as usize;
//         let div_duration = bar_duration / 3;

//         // First sample: note 0 presses
//         instrument.step_track(0);
//         assert!(
//             !matches!(instrument.keys[0].press_state, PressState::Idle),
//             "Note 0 should be pressed"
//         );

//         // After first division: note 0 releases, rest starts (no new voice)
//         instrument.step_track(div_duration);
//         // Note 0 should be released (still in active but in released state for decay)
//         assert!(
//             matches!(instrument.keys[0].press_state, PressState::Released { .. }),
//             "Note 0 should be released"
//         );

//         // After second division: rest ends, note 1 presses
//         instrument.step_track(div_duration * 2);
//         assert!(
//             !matches!(instrument.keys[1].press_state, PressState::Idle),
//             "Note 1 should be pressed"
//         );
//     }

//     #[test]
//     fn test_nested_sequence_subdivides_correctly() {
//         let track_str = "((0/1)/(2/3))";
//         let track = crate::track::parse(track_str).expect("Failed to parse track");

//         let mut instrument = Instrument::new(chromatic(), || {
//             let mut voice = crate::sin().build();
//             move |key| voice.output(key)
//         });

//         instrument.load_track(track, 120.0);

//         let bar_duration = (44100.0 * 60.0 / 120.0 * 4.0) as usize;
//         let quarter_duration = bar_duration / 4;

//         // First quarter: note 0 presses
//         instrument.step_track(0);
//         assert!(!matches!(instrument.keys[0].press_state, PressState::Idle));

//         // Second quarter: note 0 releases, note 1 presses
//         instrument.step_track(quarter_duration);
//         assert!(!matches!(instrument.keys[1].press_state, PressState::Idle));

//         // Third quarter: note 1 releases, note 2 presses
//         instrument.step_track(quarter_duration * 2);
//         assert!(!matches!(instrument.keys[2].press_state, PressState::Idle));

//         // Fourth quarter: note 2 releases, note 3 presses
//         instrument.step_track(quarter_duration * 3);
//         assert!(!matches!(instrument.keys[3].press_state, PressState::Idle));
//     }

//     #[test]
//     fn test_multiple_notes_pressed_released_same_sample() {
//         let track_str = "(0/1)(2/3)";
//         let track = crate::track::parse(track_str).expect("Failed to parse track");

//         let mut instrument = Instrument::new(chromatic(), || {
//             let mut voice = crate::sin().build();
//             move |key| voice.output(key)
//         });

//         instrument.load_track(track, 120.0);

//         let bar_duration = (44100.0 * 60.0 / 120.0 * 4.0) as usize;
//         let div_duration = bar_duration / 2;

//         // At sample 0: note 0 presses
//         instrument.step_track(0);
//         assert!(!matches!(instrument.keys[0].press_state, PressState::Idle));

//         // At first bar division boundary: note 0 releases, note 1 presses
//         instrument.step_track(div_duration);
//         assert!(
//             !matches!(instrument.keys[1].press_state, PressState::Idle),
//             "Note 1 should be pressed after first division"
//         );

//         // At bar boundary: note 1 releases and note 2 presses (same sample)
//         instrument.step_track(bar_duration);
//         assert!(
//             !matches!(instrument.keys[2].press_state, PressState::Idle),
//             "Note 2 should be pressed at bar boundary"
//         );
//     }

//     #[test]
//     fn test_same_note_pressed_twice_press_state_updated() {
//         let track_str = "(0/2/0/4)";
//         let track = crate::track::parse(track_str).expect("Failed to parse track");

//         let mut instrument = Instrument::new(chromatic(), || {
//             let mut voice = crate::sin().build();
//             move |key| voice.output(key)
//         });

//         instrument.load_track(track, 120.0);

//         instrument.step_track(0); // Note 0 presses

//         let bar_duration = (44100.0 * 60.0 / 120.0 * 4.0) as usize;
//         let div_duration = bar_duration / 4;

//         instrument.step_track(div_duration); // Note 2 presses, Note 0 should be released

//         // Check that note 0's press_state is Released
//         match instrument.keys[0].press_state {
//             PressState::Released {
//                 pressed_at,
//                 released_at,
//                 ..
//             } => {
//                 assert!(
//                     pressed_at < released_at,
//                     "pressed_at should be before released_at"
//                 );
//             }
//             PressState::Pressed { .. } => {
//                 panic!("Note 0's press_state should be Released, not Pressed!");
//             }
//             PressState::Idle => {
//                 panic!("Note 0's press_state should be Released, not Idle!");
//             }
//         }
//     }

//     #[test]
//     fn test_symphony_generates_equal_presses_and_releases() {
//         use std::collections::HashMap;

//         let track_str = "(0/0/0/0)(2/2/2/2)(4/4/4/4)";

//         let track = crate::track::parse(track_str).expect("Failed to parse track");
//         let mut instrument = Instrument::new(chromatic(), || {
//             let mut voice = crate::sin().build();
//             move |key| voice.output(key)
//         });
//         instrument.load_track(track, 80.0);

//         // Count presses and releases for each pitch
//         let mut press_counts: HashMap<u8, i32> = HashMap::new();
//         let mut release_counts: HashMap<u8, i32> = HashMap::new();

//         for event in &instrument.event_buffer {
//             match event {
//                 NoteEvent::Press(pitch) => {
//                     *press_counts.entry(*pitch).or_insert(0) += 1;
//                 }
//                 NoteEvent::Release(pitch) => {
//                     *release_counts.entry(*pitch).or_insert(0) += 1;
//                 }
//             }
//         }

//         // Every pitch that was pressed should be released the same number of times
//         for (pitch, press_count) in &press_counts {
//             let release_count = release_counts.get(pitch).unwrap_or(&0);
//             assert_eq!(
//                 press_count, release_count,
//                 "Pitch {} has {} presses but {} releases",
//                 pitch, press_count, release_count
//             );
//         }
//     }
// }
