use crate::Signal;
use std::array;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    Idle,
    Pressed {
        pressed_at: usize,
    },
    Released {
        pressed_at: usize,
        released_at: usize,
    },
}

pub struct Key {
    pub pitch: u8,
    pub freq: f32,
    pub state: KeyState,
}

impl Key {
    pub fn pressed(&self) -> bool {
        matches!(self.state, KeyState::Pressed { .. })
    }
}

pub struct Keyboard<V> {
    voices: [V; 128],
    states: [KeyState; 128],
}

impl<V: Default> Keyboard<V> {
    pub fn new() -> Self {
        Keyboard {
            voices: array::from_fn(|_| V::default()),
            states: array::from_fn(|_| KeyState::Idle),
        }
    }
}

impl<V> Keyboard<V> {
    pub fn with_builder(builder: impl Fn() -> V) -> Self {
        Keyboard {
            voices: array::from_fn(|_| builder()),
            states: array::from_fn(|_| KeyState::Idle),
        }
    }

    pub fn update(&mut self, events: Vec<crate::NoteEvent>, signal: &Signal) {
        for event in events {
            match event {
                crate::NoteEvent::Press { pitch } => {
                    self.states[pitch as usize] = KeyState::Pressed {
                        pressed_at: signal.position,
                    };
                }
                crate::NoteEvent::Release { pitch } => {
                    if let KeyState::Pressed { pressed_at } = self.states[pitch as usize] {
                        self.states[pitch as usize] = KeyState::Released {
                            pressed_at,
                            released_at: signal.position,
                        };
                    }
                }
            }
        }
    }

    pub fn per_key(&mut self, mut f: impl FnMut(&mut V, Key)) {
        for pitch in 0..128u8 {
            let frequency = 440.0 * 2.0_f32.powf((pitch as f32 - 69.0) / 12.0);
            let key = Key {
                pitch,
                freq: frequency,
                state: self.states[pitch as usize],
            };
            f(&mut self.voices[pitch as usize], key);
        }
    }
}

impl<V: Default> Default for Keyboard<V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_update_press() {
        let mut keyboard: Keyboard<()> = Keyboard::new();
        let signal = Signal::new(44100);
        let events = vec![crate::NoteEvent::Press { pitch: 60 }];

        keyboard.update(events, &signal);

        assert!(matches!(
            keyboard.states[60],
            KeyState::Pressed { pressed_at: 0 }
        ));
    }

    #[test]
    fn test_keyboard_update_release() {
        let mut keyboard: Keyboard<()> = Keyboard::new();
        let mut signal = Signal::new(44100);

        let press_events = vec![crate::NoteEvent::Press { pitch: 60 }];
        keyboard.update(press_events, &signal);

        signal.position = 1000;
        let release_events = vec![crate::NoteEvent::Release { pitch: 60 }];
        keyboard.update(release_events, &signal);

        assert!(matches!(
            keyboard.states[60],
            KeyState::Released {
                pressed_at: 0,
                released_at: 1000
            }
        ));
    }

    #[test]
    fn test_keyboard_per_key_iteration() {
        let mut keyboard: Keyboard<i32> = Keyboard::with_builder(|| 42);
        let signal = Signal::new(44100);
        let events = vec![crate::NoteEvent::Press { pitch: 60 }];
        keyboard.update(events, &signal);

        let mut visited_pitches = Vec::new();
        keyboard.per_key(|_voice, key| {
            visited_pitches.push(key.pitch);
        });

        assert_eq!(visited_pitches.len(), 128);
        assert!(visited_pitches.contains(&60));
    }

    #[test]
    fn test_keyboard_per_key_with_state() {
        let mut keyboard: Keyboard<String> = Keyboard::with_builder(|| "voice".to_string());
        let signal = Signal::new(44100);
        let events = vec![crate::NoteEvent::Press { pitch: 60 }];
        keyboard.update(events, &signal);

        keyboard.per_key(|voice, key| {
            if key.pitch == 60 {
                assert!(matches!(key.state, KeyState::Pressed { pressed_at: 0 }));
                assert_eq!(voice, "voice");
            }
        });
    }
}
