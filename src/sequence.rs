use serde::{Deserialize, Serialize};
use std::num::NonZeroU16;

use crate::compile::PatchControls;
use crate::scale::Scale;
use crate::time::{Hertz, SampleRate};

mod notation;
pub use notation::parse_sequence;

pub const VOICES: usize = 6;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Stroke(Vec<[f32; 3]>);

impl<'de> Deserialize<'de> for Stroke {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::try_from(Vec::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<Vec<[f32; 3]>> for Stroke {
    type Error = &'static str;

    fn try_from(points: Vec<[f32; 3]>) -> Result<Self, Self::Error> {
        if points.len() < 2
            || points.iter().any(|[time, pitch, expression]| {
                !time.is_finite()
                    || *time < 0.0
                    || !(0.0..=127.0).contains(pitch)
                    || !(-1.0..=1.0).contains(expression)
            })
            || points.windows(2).any(|pair| pair[0][0] >= pair[1][0])
        {
            return Err(
                "a stroke needs ordered finite times, MIDI pitches, and expression in -1..1",
            );
        }
        Ok(Self(points))
    }
}

impl Stroke {
    pub fn points(&self) -> &[[f32; 3]] {
        &self.0
    }

    fn sample(&self, time: f32) -> Option<[f32; 2]> {
        if !time.is_finite() || time < self.0[0][0] || time >= self.0.last()?[0] {
            return None;
        }
        let index = self.0.partition_point(|point| point[0] <= time);
        let a = self.0[index - 1];
        let b = self.0[index];
        let fraction = (time - a[0]) / (b[0] - a[0]);
        Some([
            a[1] + (b[1] - a[1]) * fraction,
            a[2] + (b[2] - a[2]) * fraction,
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Sequence {
    strokes: Vec<Stroke>,
    bars: NonZeroU16,
}

impl Sequence {
    pub fn new(strokes: Vec<Stroke>, bars: u16) -> Result<Self, &'static str> {
        let bars = NonZeroU16::new(bars).ok_or("A sequence needs at least one bar")?;
        for stroke in &strokes {
            if stroke.0.last().unwrap()[0] > bars.get() as f32 {
                return Err("A stroke extends beyond the loop");
            }
            let start = stroke.0[0][0];
            if strokes
                .iter()
                .filter(|other| other.0[0][0] <= start && start < other.0.last().unwrap()[0])
                .take(VOICES + 1)
                .count()
                > VOICES
            {
                return Err("This instrument supports six simultaneous strokes");
            }
        }
        Ok(Self { strokes, bars })
    }

    pub fn strokes(&self) -> &[Stroke] {
        &self.strokes
    }

    pub fn bars(&self) -> u16 {
        self.bars.get()
    }
}

impl<'de> Deserialize<'de> for Sequence {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Input {
            strokes: Vec<Stroke>,
            bars: u16,
        }
        let input = Input::deserialize(deserializer)?;
        Self::new(input.strokes, input.bars).map_err(serde::de::Error::custom)
    }
}

pub struct Player {
    sequence: Sequence,
    starts: Vec<usize>,
    next_stroke: usize,
    scale: Scale,
    step: f64,
    time: f64,
    voices: [Voice; VOICES],
}

#[derive(Clone, Copy, Default)]
struct Voice {
    controls: PatchControls,
    stroke: Option<usize>,
}

impl Player {
    pub fn new(sequence: Sequence, bpm: NonZeroU16, rate: SampleRate, scale: Scale) -> Self {
        let mut starts = (0..sequence.strokes.len()).collect::<Vec<_>>();
        starts
            .sort_by(|a, b| sequence.strokes[*a].0[0][0].total_cmp(&sequence.strokes[*b].0[0][0]));
        Self {
            sequence,
            starts,
            next_stroke: 0,
            scale,
            step: bpm.get() as f64 / 240.0 / rate.value() as f64,
            time: 0.0,
            voices: [Voice::default(); VOICES],
        }
    }

    pub fn playhead(&self) -> f32 {
        self.time as f32
    }

    pub fn replace(&mut self, mut next: Self) -> Self {
        next.time = self.time % next.sequence.bars.get() as f64;
        next.voices = self.voices;
        next.next_stroke = 0;
        std::mem::replace(self, next)
    }

    pub fn advance(&mut self) -> [PatchControls; VOICES] {
        let next_time = self.time + self.step;
        if next_time >= self.sequence.bars.get() as f64 {
            self.next_stroke = 0;
        }
        self.time = next_time % self.sequence.bars.get() as f64;
        let time = self.time as f32;
        for voice in &mut self.voices {
            if voice.stroke.is_some_and(|index| {
                self.sequence
                    .strokes
                    .get(index)
                    .and_then(|stroke| stroke.sample(time))
                    .is_none()
            }) {
                voice.stroke = None;
                voice.controls.gate = 0.0;
            }
        }
        let mut sounding = self.voices.map(|voice| voice.stroke);
        while let Some(&index) = self.starts.get(self.next_stroke) {
            let stroke = &self.sequence.strokes[index];
            if stroke.0[0][0] > time {
                break;
            }
            self.next_stroke += 1;
            if time < stroke.0.last().unwrap()[0] && !sounding.contains(&Some(index)) {
                *sounding.iter_mut().find(|stroke| stroke.is_none()).unwrap() = Some(index);
            }
        }
        sounding.sort_unstable();
        for index in sounding.into_iter().flatten() {
            let stroke = &self.sequence.strokes[index];
            let Some([pitch, expression]) = stroke.sample(time) else {
                continue;
            };
            let slot = self
                .voices
                .iter()
                .position(|voice| voice.stroke == Some(index))
                .or_else(|| self.voices.iter().position(|voice| voice.stroke.is_none()))
                .unwrap();
            let voice = &mut self.voices[slot];
            voice.stroke = Some(index);
            let octave = ((pitch - self.scale.note(0) as f32) / 12.0).floor() as i32;
            let degree = (octave * 7..=octave * 7 + 7)
                .min_by(|a, b| {
                    (self.scale.note(*a) as f32 - pitch)
                        .abs()
                        .total_cmp(&(self.scale.note(*b) as f32 - pitch).abs())
                })
                .unwrap();
            voice.controls = PatchControls {
                frequency: Hertz::new(440.0 * 2.0_f32.powf((pitch - 69.0) / 12.0)),
                gate: 1.0,
                degree,
                expression,
            };
        }
        self.voices.map(|voice| voice.controls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::cmin;

    #[test]
    fn scheduled_playback_matches_scanning_through_loops_and_replacement() {
        fn scanning(player: &mut Player) -> [PatchControls; VOICES] {
            player.time = (player.time + player.step) % player.sequence.bars.get() as f64;
            let time = player.time as f32;
            for voice in &mut player.voices {
                if voice.stroke.is_some_and(|index| {
                    player
                        .sequence
                        .strokes
                        .get(index)
                        .and_then(|stroke| stroke.sample(time))
                        .is_none()
                }) {
                    voice.stroke = None;
                    voice.controls.gate = 0.0;
                }
            }
            for (index, stroke) in player.sequence.strokes.iter().enumerate() {
                let Some([pitch, expression]) = stroke.sample(time) else {
                    continue;
                };
                let slot = player
                    .voices
                    .iter()
                    .position(|voice| voice.stroke == Some(index))
                    .or_else(|| {
                        player
                            .voices
                            .iter()
                            .position(|voice| voice.stroke.is_none())
                    })
                    .unwrap();
                let voice = &mut player.voices[slot];
                voice.stroke = Some(index);
                let octave = ((pitch - player.scale.note(0) as f32) / 12.0).floor() as i32;
                let degree = (octave * 7..=octave * 7 + 7)
                    .min_by(|a, b| {
                        (player.scale.note(*a) as f32 - pitch)
                            .abs()
                            .total_cmp(&(player.scale.note(*b) as f32 - pitch).abs())
                    })
                    .unwrap();
                voice.controls = PatchControls {
                    frequency: Hertz::new(440.0 * 2.0_f32.powf((pitch - 69.0) / 12.0)),
                    gate: 1.0,
                    degree,
                    expression,
                };
            }
            player.voices.map(|voice| voice.controls)
        }
        let make = |reverse: bool| {
            let mut strokes = (0..36)
                .map(|index| {
                    let start = (index / 6) as f32 * 0.15;
                    Stroke::try_from(vec![
                        [start, 48.0 + index as f32, -0.5],
                        [start + 0.1, 49.0 + index as f32, 0.75],
                    ])
                    .unwrap()
                })
                .collect::<Vec<_>>();
            if reverse {
                strokes.reverse();
            }
            Player::new(
                Sequence::new(strokes, 1).unwrap(),
                NonZeroU16::new(120).unwrap(),
                SampleRate::new(100).unwrap(),
                cmin(),
            )
        };
        let mut scheduled = make(true);
        let mut reference = make(true);
        for frame in 0..1000 {
            if frame == 333 {
                let next = make(false);
                let retired = scheduled.replace(next);
                drop(retired);
                let retired = reference.replace(make(false));
                drop(retired);
            }
            let actual = assert_no_alloc::assert_no_alloc(|| scheduled.advance());
            assert_eq!(actual, scanning(&mut reference), "frame {frame}");
        }
    }

    #[test]
    fn notation_preserves_weighted_nested_polyphonic_timing() {
        for (notation, bars, count) in [
            ("(0)", 1, 1),
            ("(0/_/2)", 1, 2),
            ("(_***/0)", 1, 1),
            ("((0/_/1/2)/(0/_/1))", 1, 5),
            ("(0)(1)(2)", 3, 3),
            ("{(0/1)&(2/3)}", 1, 4),
            ("{(0/2/4)&(1/3)}", 1, 5),
            ("(({0&2}/{1&3})/4)", 1, 5),
            ("{0/2/4}", 1, 3),
            ("(0/2/0/4){0/2&2/4}", 2, 8),
            (
                "{ {0&3&5}/{1&3&5&7}/{2&4&6}/{1&3&6} & (12/13/14/(15/16/15/14)/12) }",
                1,
                21,
            ),
            ("# intro\n (0) # first bar\n (2)", 2, 2),
        ] {
            let sequence = parse_sequence(notation, &cmin()).unwrap();
            assert_eq!(sequence.bars(), bars, "{notation}");
            assert_eq!(sequence.strokes().len(), count, "{notation}");
        }
        let sequence = parse_sequence("(0+**/(-1/2-))(_/4)", &cmin()).unwrap();
        let expected = [
            [[0.0, 49.0, 1.0], [0.75, 49.0, 1.0]],
            [[0.75, 46.0, 1.0], [0.875, 46.0, 1.0]],
            [[0.875, 50.0, 1.0], [1.0, 50.0, 1.0]],
            [[1.5, 55.0, 1.0], [2.0, 55.0, 1.0]],
        ];
        for (stroke, expected) in sequence.strokes().iter().zip(expected) {
            assert_eq!(stroke.points(), &expected);
        }
        let extremes = parse_sequence("(-2147483648/2147483647)", &cmin()).unwrap();
        assert_eq!(extremes.strokes()[0].points()[0][1], 0.0);
        assert_eq!(extremes.strokes()[1].points()[0][1], 127.0);
    }

    #[test]
    fn sequence_parsing_counts_simultaneous_not_total_strokes() {
        let strokes: Vec<_> = (0..20)
            .map(|index| {
                Stroke::try_from(vec![
                    [index as f32, 60.0, 1.0],
                    [index as f32 + 1.0, 60.0, 1.0],
                ])
                .unwrap()
            })
            .collect();
        assert!(Sequence::new(strokes, 20).is_ok());
        let stroke = Stroke::try_from(vec![[0.0, 60.0, 1.0], [1.0, 60.0, 1.0]]).unwrap();
        assert!(Sequence::new(vec![stroke.clone(); VOICES], 1).is_ok());
        assert!(Sequence::new(vec![stroke; VOICES + 1], 1).is_err());
    }

    #[test]
    fn replacement_preserves_bar_position_and_returns_retired_storage_without_allocating() {
        let mut player = Player::new(
            parse_sequence("(0)(2)(4)", &cmin()).unwrap(),
            NonZeroU16::new(120).unwrap(),
            SampleRate::new(1000).unwrap(),
            cmin(),
        );
        player.time = 2.25;
        assert_eq!(player.advance()[0].gate, 1.0);
        let previous = player.time;
        let next = Player::new(
            parse_sequence("(7)", &cmin()).unwrap(),
            NonZeroU16::new(60).unwrap(),
            SampleRate::new(1000).unwrap(),
            cmin(),
        );
        let retired = assert_no_alloc::assert_no_alloc(|| player.replace(next));
        assert_eq!(retired.time, previous);
        assert_eq!(player.time, previous % 1.0);
        let controls = assert_no_alloc::assert_no_alloc(|| player.advance());
        assert_eq!(controls[0].gate, 1.0);
        assert_eq!(controls[0].degree, 7);
        assert!((player.time - previous % 1.0 - 0.00025).abs() < 1e-12);
    }

    #[test]
    fn overlapping_same_pitch_strokes_keep_independent_voices() {
        let sequence = Sequence::new(
            vec![
                Stroke::try_from(vec![[0.0, 60.0, -1.0], [0.5, 60.0, -1.0]]).unwrap(),
                Stroke::try_from(vec![[0.25, 60.0, 1.0], [0.75, 60.0, 1.0]]).unwrap(),
            ],
            1,
        )
        .unwrap();
        let mut runtime = Player::new(
            sequence,
            NonZeroU16::new(120).unwrap(),
            SampleRate::new(1000).unwrap(),
            cmin(),
        );
        runtime.time = 0.3;
        let controls = runtime.advance();
        assert_eq!(
            controls
                .iter()
                .filter(|control| control.gate == 1.0)
                .count(),
            2
        );
        assert_eq!(controls[0].expression, -1.0);
        assert_eq!(controls[1].expression, 1.0);
        runtime.time = 0.6;
        let controls = runtime.advance();
        assert_eq!(controls[0].gate, 0.0);
        assert_eq!(controls[1].gate, 1.0);
    }

    #[test]
    fn deserialization_cannot_create_unplayable_sequences() {
        for input in [
            "(strokes: [[(0.0, 60.0, 0.0), (2.0, 60.0, 0.0)]], bars: 1)",
            "(strokes: [], bars: 0)",
        ] {
            assert!(ron::from_str::<Sequence>(input).is_err(), "{input}");
        }
        let stroke = "[(0.0, 60.0, 0.0), (1.0, 60.0, 0.0)]";
        let input = format!("(strokes: [{}], bars: 1)", [stroke; 7].join(","));
        assert!(ron::from_str::<Sequence>(&input).is_err());
    }

    #[test]
    fn notation_consumes_the_entire_input_without_panicking() {
        for input in ["(0)garbage", "(0)(", "(99999999999999999999)"] {
            assert!(parse_sequence(input, &crate::scale::cmin()).is_err());
        }
    }

    #[test]
    fn legacy_import_preserves_overlapping_same_pitch_durations() {
        let sequence = parse_sequence("{0&(_/0/_)}", &crate::scale::cmin()).unwrap();
        assert_eq!(sequence.strokes.len(), 2);
        assert_eq!(sequence.strokes[0].points()[0][0], 0.0);
        assert_eq!(sequence.strokes[0].points()[1][0], 1.0);
        assert_eq!(sequence.strokes[1].points()[0][0], 1.0 / 3.0);
        assert_eq!(sequence.strokes[1].points()[1][0], 2.0 / 3.0);
    }

    #[test]
    fn sustained_pitch_and_expression_interpolate_without_retriggering() {
        let stroke = Stroke::try_from(vec![[0.25, 60.2, -1.0], [0.75, 72.8, 1.0]]).unwrap();
        assert_eq!(stroke.sample(0.0), None);
        assert_eq!(stroke.sample(0.25), Some([60.2, -1.0]));
        assert_eq!(stroke.sample(0.5), Some([66.5, 0.0]));
        assert_eq!(stroke.sample(0.75), None);
        assert_eq!(stroke.sample(f32::NAN), None);
    }

    #[test]
    fn rejects_invalid_strokes_at_construction_and_deserialization() {
        for points in [
            vec![],
            vec![[0.0, 60.0, 0.0]],
            vec![[0.0, 60.0, 0.0], [0.0, 61.0, 0.0]],
            vec![[1.0, 60.0, 0.0], [0.0, 61.0, 0.0]],
            vec![[0.0, f32::NAN, 0.0], [1.0, 61.0, 0.0]],
            vec![[0.0, 60.0, 2.0], [1.0, 61.0, 0.0]],
        ] {
            assert!(Stroke::try_from(points.clone()).is_err());
            assert!(ron::from_str::<Stroke>(&ron::to_string(&points).unwrap()).is_err());
        }
    }

    #[test]
    fn project_roundtrip_preserves_free_pitch_and_expression() {
        let sequence = Sequence::new(
            vec![Stroke::try_from(vec![[0.13, 60.123, -0.72], [0.97, 63.88, 0.91]]).unwrap()],
            1,
        )
        .unwrap();
        let restored: Sequence = ron::from_str(&ron::to_string(&sequence).unwrap()).unwrap();
        assert_eq!(sequence, restored);
    }
}
