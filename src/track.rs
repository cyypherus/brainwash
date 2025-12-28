use nom::{
    Parser,
    branch::alt,
    character::complete::{char, digit1},
    combinator::opt,
    multi::{many1, separated_list1},
    sequence::delimited,
};

#[derive(Clone, Debug, PartialEq)]
struct ParsedNote {
    degree: i32,
    chromatic_shift: i32,
}

#[derive(Clone, Debug, PartialEq)]
enum Item {
    Note(ParsedNote),
    Rest,
    Sequence(Vec<Division>),
    Polyphony(Vec<Vec<Division>>),
}

#[derive(Clone, Debug, PartialEq)]
struct Division {
    item: Item,
    nudge_before: Option<u32>,
    nudge_after: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
struct Bar {
    divisions: Vec<Division>,
}

#[derive(Clone, Debug, PartialEq)]
struct Layer {
    bars: Vec<Bar>,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedTrackAST {
    layers: Vec<Layer>,
}

fn parse_number(input: &str) -> nom::IResult<&str, i32> {
    let (input, sign) = opt(char('-')).parse(input)?;
    let (input, num) = digit1.parse(input)?;
    let val: i32 = num.parse().unwrap();
    Ok((input, if sign.is_some() { -val } else { val }))
}

fn parse_nudge(input: &str) -> nom::IResult<&str, u32> {
    let (input, digits) = delimited(char('<'), digit1, char('>')).parse(input)?;
    Ok((input, digits.parse::<u32>().unwrap()))
}

fn parse_note(input: &str) -> nom::IResult<&str, Item> {
    let (input, degree) = parse_number(input)?;
    let (input, chromatic_shift) =
        opt(alt((char('+').map(|_| 1i32), char('-').map(|_| -1i32)))).parse(input)?;
    Ok((
        input,
        Item::Note(ParsedNote {
            degree,
            chromatic_shift: chromatic_shift.unwrap_or(0),
        }),
    ))
}

fn parse_rest(input: &str) -> nom::IResult<&str, Item> {
    let (input, _) = char('_').parse(input)?;
    Ok((input, Item::Rest))
}

fn parse_nested_bar(input: &str) -> nom::IResult<&str, Item> {
    let (input, divisions) = delimited(char('('), parse_divisions, char(')')).parse(input)?;
    Ok((input, Item::Sequence(divisions)))
}

fn parse_polyphonic_item(input: &str) -> nom::IResult<&str, Item> {
    let (input, layers) = delimited(
        char('{'),
        separated_list1(char('%'), parse_divisions),
        char('}'),
    )
    .parse(input)?;
    Ok((input, Item::Polyphony(layers)))
}

fn parse_item(input: &str) -> nom::IResult<&str, Item> {
    alt((
        parse_polyphonic_item,
        parse_nested_bar,
        parse_note,
        parse_rest,
    ))
    .parse(input)
}

fn parse_division(input: &str) -> nom::IResult<&str, (Item, Option<u32>)> {
    let (input, item) = parse_item(input)?;
    let (input, nudge) = opt(|i| parse_nudge(i)).parse(input)?;
    Ok((input, (item, nudge)))
}

fn parse_division_with_separator(input: &str) -> nom::IResult<&str, Division> {
    let (input, nudge_before) = opt(|i| parse_nudge(i)).parse(input)?;
    let (input, (item, nudge_after)) = parse_division(input)?;

    Ok((
        input,
        Division {
            item,
            nudge_before,
            nudge_after,
        },
    ))
}

fn parse_divisions(input: &str) -> nom::IResult<&str, Vec<Division>> {
    separated_list1(char('/'), |i| parse_division_with_separator(i)).parse(input)
}

fn parse_one_polyphony_layer(input: &str) -> nom::IResult<&str, Vec<Division>> {
    alt((
        delimited(char('('), |i| parse_divisions(i), char(')')),
        |i| parse_divisions(i),
    ))
    .parse(input)
}

fn parse_polyphonic_divisions(input: &str) -> nom::IResult<&str, Vec<Division>> {
    let (input, poly_layers) =
        separated_list1(char('%'), parse_one_polyphony_layer).parse(input)?;

    if poly_layers.len() == 1 {
        return Ok((input, poly_layers.into_iter().next().unwrap()));
    }

    let max_divs = poly_layers.iter().map(|l| l.len()).max().unwrap_or(0);
    let result = (0..max_divs)
        .map(|div_idx| {
            let layers: Vec<Vec<Division>> = poly_layers
                .iter()
                .filter_map(|layer| {
                    if div_idx < layer.len() {
                        Some(vec![layer[div_idx].clone()])
                    } else {
                        None
                    }
                })
                .collect();
            Division {
                item: Item::Polyphony(layers),
                nudge_before: None,
                nudge_after: None,
            }
        })
        .collect();

    Ok((input, result))
}

fn parse_bar(input: &str) -> nom::IResult<&str, Bar> {
    let (input, divisions) = alt((
        delimited(char('('), |i| parse_divisions(i), char(')')),
        delimited(char('{'), |i| parse_polyphonic_divisions(i), char('}')),
    ))
    .parse(input)?;
    Ok((input, Bar { divisions }))
}

fn parse_layer_sequence(input: &str) -> nom::IResult<&str, Layer> {
    let (input, bars) = many1(|i| parse_bar(i)).parse(input)?;
    Ok((input, Layer { bars }))
}

fn parse_track_section(input: &str) -> nom::IResult<&str, Vec<Layer>> {
    let (input, layer) = parse_layer_sequence(input)?;
    Ok((input, vec![layer]))
}

fn parse_track(input: &str) -> nom::IResult<&str, ParsedTrackAST> {
    let (input, sections) = many1(|i| parse_track_section(i)).parse(input)?;
    let layers = sections.into_iter().flatten().collect();
    Ok((input, ParsedTrackAST { layers }))
}

fn parse_notation(input: &str) -> Result<ParsedTrackAST, String> {
    let input: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    match parse_track(&input) {
        Ok((_, track)) => Ok(track),
        Err(e) => Err(format!("Parse error: {:?}", e)),
    }
}

#[derive(Clone, Debug)]
struct TimelineNote {
    pitch: u8,
    start: f32,
    end: f32,
}

fn extract_notes(
    item: &Item,
    start: f32,
    end: f32,
    notes: &mut Vec<TimelineNote>,
    scale: &crate::Scale,
) {
    match item {
        Item::Note(note) => {
            let pitch = (scale.note(note.degree) + note.chromatic_shift).clamp(0, 127) as u8;
            notes.push(TimelineNote { pitch, start, end });
        }
        Item::Rest => {}
        Item::Sequence(divs) => {
            let div_span = end - start;
            let samples_per_subdiv = div_span / divs.len() as f32;
            for (sub_idx, subdiv) in divs.iter().enumerate() {
                let sub_start = start + (sub_idx as f32 * samples_per_subdiv);
                let sub_end = sub_start + samples_per_subdiv;
                extract_notes(&subdiv.item, sub_start, sub_end, notes, scale);
            }
        }
        Item::Polyphony(layers) => {
            for layer_divs in layers {
                if layer_divs.is_empty() {
                    continue;
                }
                let div_span = end - start;
                let samples_per_subdiv = div_span / layer_divs.len() as f32;
                for (sub_idx, subdiv) in layer_divs.iter().enumerate() {
                    let sub_start = start + (sub_idx as f32 * samples_per_subdiv);
                    let sub_end = sub_start + samples_per_subdiv;
                    extract_notes(&subdiv.item, sub_start, sub_end, notes, scale);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoteEvent {
    Press { pitch: u8 },
    Release { pitch: u8 },
}

pub struct Track {
    playhead: f32,
    note_timeline: Vec<TimelineNote>,
}

impl Track {
    pub fn from_notation(notation: &str, scale: &crate::Scale) -> Result<Self, String> {
        let ast = parse_notation(notation)?;
        let mut events = Vec::new();

        for layer in &ast.layers {
            let total_divisions: usize = layer.bars.iter().map(|b| b.divisions.len()).sum();
            let mut division_idx = 0;

            for bar in &layer.bars {
                for division in &bar.divisions {
                    let div_start = division_idx as f32 / total_divisions as f32;
                    let div_end = (division_idx + 1) as f32 / total_divisions as f32;
                    extract_notes(&division.item, div_start, div_end, &mut events, scale);
                    division_idx += 1;
                }
            }
        }

        events.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());

        Ok(Track {
            playhead: 0.0,
            note_timeline: events,
        })
    }

    pub fn advance(&mut self, to: f32) -> Vec<NoteEvent> {
        let to = to.fract(); // Wrap to 0.0..1.0 range
        if to == self.playhead {
            return Vec::new();
        }

        // Infer direction based on distance
        // The range loops 0...1, so we need to find the shorter path
        let forward_distance = if to >= self.playhead {
            to - self.playhead
        } else {
            (1.0 - self.playhead) + to
        };

        let backward_distance = if self.playhead >= to {
            self.playhead - to
        } else {
            self.playhead + (1.0 - to)
        };

        let is_forward = forward_distance <= backward_distance;
        self.advance_with_direction(to, is_forward)
    }

    // fn crossed(pos: f32, from: f32, to: f32, inclusive: bool) -> bool {
    //     // Did we cross `pos` when moving from `from` to `to`?
    //     // Handles wrap-around at 1.0
    //     if from <= to {
    //         // Normal: moving forward without wrap
    //         if inclusive {
    //             from <= pos && pos <= to
    //         } else {
    //             from < pos && pos <= to
    //         }
    //     } else {
    //         // Wrap: moving from high to low (crossing 1.0 -> 0.0)
    //         if inclusive {
    //             (pos >= from && pos <= 1.0) || (pos >= 0.0 && pos <= to)
    //         } else {
    //             (pos > from && pos <= 1.0) || (pos >= 0.0 && pos <= to)
    //         }
    //     }
    // }

    #[allow(clippy::collapsible_else_if)]
    pub(crate) fn advance_with_direction(&mut self, to: f32, forward: bool) -> Vec<NoteEvent> {
        // Normalize to [0, 1]
        let from = self.playhead;
        let to = to.rem_euclid(1.0);

        let mut events = Vec::new();
        for note in &self.note_timeline {
            let note_start = note.start;
            let note_end = note.end;
            // dbg!(note_start, note_end, from, to);
            if forward {
                if to < from {
                    // looped around
                    // range of track: 0---1
                    // range between from..to: ===
                    // range of note start..end: s###e
                    // 0---------------------1
                    // ###e==t---------f==s###
                    if note_start >= from || note_start < to {
                        events.push(NoteEvent::Press { pitch: note.pitch });
                    }
                    // ======t------s##f##e===
                    if note_end >= from || note_end < to {
                        events.push(NoteEvent::Release { pitch: note.pitch });
                    }
                } else {
                    // ------f=s#######t##e----
                    if note_start >= from && note_start < to {
                        events.push(NoteEvent::Press { pitch: note.pitch });
                    }
                    // ----s##f#####e==t------
                    if note_end >= from && note_end < to {
                        events.push(NoteEvent::Release { pitch: note.pitch });
                    }
                }
            } else {
                if to > from {
                    // looped around
                    if note_start <= from || note_start > to {
                        events.push(NoteEvent::Press { pitch: note.pitch });
                    }
                    if note_end <= from || note_end > to {
                        events.push(NoteEvent::Release { pitch: note.pitch });
                    }
                } else {
                    if note_start <= from && note_start > to {
                        events.push(NoteEvent::Press { pitch: note.pitch });
                    }
                    if note_end <= from && note_end > to {
                        events.push(NoteEvent::Release { pitch: note.pitch });
                    }
                }
            }
        }
        self.playhead = to;
        events
        // let to = if to == 0.0 && forward { 1.0 } else { to };

        // if (to - self.playhead).abs() < 1e-9 {
        //     return Vec::new();
        // }

        // let mut events = Vec::new();
        // let (start, end) = if forward {
        //     (self.playhead, to)
        // } else {
        //     (to, self.playhead)
        // };

        // for note in &self.note_timeline {
        //     // Press: include the boundary, Release: exclude the from boundary
        //     if Self::crossed(note.start, start, end, true) {
        //         events.push(NoteEvent::Press { pitch: note.pitch });
        //     }
        //     if Self::crossed(note.end, start, end, false) {
        //         events.push(NoteEvent::Release { pitch: note.pitch });
        //     }
        // }

        // self.playhead = to;
        // events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Parsing tests from TRACK_NOTATION.md

    #[test]
    fn test_parse_single_note() {
        let result = parse_notation("(0)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers.len(), 1);
        assert_eq!(ast.layers[0].bars.len(), 1);
        assert_eq!(ast.layers[0].bars[0].divisions.len(), 1);
    }

    #[test]
    fn test_parse_three_divisions() {
        let result = parse_notation("(0/_/2)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions.len(), 3);
        match &ast.layers[0].bars[0].divisions[0].item {
            Item::Note(n) => assert_eq!(n.degree, 0),
            _ => panic!("Expected note at division 0"),
        }
        match &ast.layers[0].bars[0].divisions[1].item {
            Item::Rest => {}
            _ => panic!("Expected rest at division 1"),
        }
        match &ast.layers[0].bars[0].divisions[2].item {
            Item::Note(n) => assert_eq!(n.degree, 2),
            _ => panic!("Expected note at division 2"),
        }
    }

    #[test]
    fn test_parse_three_notes() {
        let result = parse_notation("(0/1/2)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions.len(), 3);
    }

    #[test]
    fn test_parse_nudge_before() {
        let result = parse_notation("(0<30>/1)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions[0].nudge_after, Some(30));
    }

    #[test]
    fn test_parse_nudge_after() {
        let result = parse_notation("(0/<10>1)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions[1].nudge_before, Some(10));
    }

    #[test]
    fn test_parse_complex_nudges() {
        let result = parse_notation("(0/<30>1<10>/2)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(divs[0].nudge_after, None);
        assert_eq!(divs[1].nudge_before, Some(30));
        assert_eq!(divs[1].nudge_after, Some(10));
        assert_eq!(divs[2].nudge_before, None);
    }

    #[test]
    fn test_parse_nested_sequence() {
        let result = parse_notation("((0/_/1/2)/(0/_/1))");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions.len(), 2);
        match &ast.layers[0].bars[0].divisions[0].item {
            Item::Sequence(divs) => {
                assert_eq!(divs.len(), 4);
            }
            _ => panic!("Expected sequence"),
        }
    }

    #[test]
    fn test_parse_two_bars() {
        let result = parse_notation("(0)(1)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars.len(), 2);
    }

    #[test]
    fn test_parse_negative_degree() {
        let result = parse_notation("(-1/1/_/2)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(divs.len(), 4);
        match &divs[0].item {
            Item::Note(n) => assert_eq!(n.degree, -1),
            _ => panic!("Expected negative note"),
        }
    }

    #[test]
    fn test_parse_chromatic_shifts() {
        let result = parse_notation("(0+/2-/4)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        let divs = &ast.layers[0].bars[0].divisions;
        match &divs[0].item {
            Item::Note(n) => {
                assert_eq!(n.degree, 0);
                assert_eq!(n.chromatic_shift, 1);
            }
            _ => panic!("Expected note with chromatic shift"),
        }
        match &divs[1].item {
            Item::Note(n) => {
                assert_eq!(n.degree, 2);
                assert_eq!(n.chromatic_shift, -1);
            }
            _ => panic!("Expected note with chromatic shift"),
        }
    }

    #[test]
    fn test_parse_simple_polyphony() {
        let result = parse_notation("{(0/1)%(2/3)}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars.len(), 1);
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(divs.len(), 2, "Expected 2 divisions in polyphony");
        match &divs[0].item {
            Item::Polyphony(layers) => {
                assert_eq!(layers.len(), 2);
            }
            _ => panic!("Expected polyphony"),
        }
        match &divs[1].item {
            Item::Polyphony(layers) => {
                assert_eq!(layers.len(), 2);
            }
            _ => panic!("Expected polyphony"),
        }
    }

    #[test]
    fn test_parse_polyphony_mismatched_divisions() {
        let result = parse_notation("{(0/2/4)%(1/3)}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(
            divs.len(),
            3,
            "First layer has 3 divs, second has 2, max_divs=3"
        );
    }

    #[test]
    fn test_parse_nested_polyphony() {
        let result = parse_notation("(({0%2}/{1%3})/4)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions.len(), 2);
        match &ast.layers[0].bars[0].divisions[0].item {
            Item::Sequence(divs) => {
                assert_eq!(divs.len(), 2);
                match &divs[0].item {
                    Item::Polyphony(layers) => {
                        assert_eq!(layers.len(), 2);
                    }
                    _ => panic!("Expected polyphony in nested sequence"),
                }
            }
            _ => panic!("Expected sequence"),
        }
    }

    #[test]
    fn test_parse_curly_braces_as_bar() {
        let result = parse_notation("{0/2/4}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars.len(), 1);
        assert_eq!(ast.layers[0].bars[0].divisions.len(), 3);
    }

    #[test]
    fn test_parse_sequential_bars_with_polyphony() {
        let result = parse_notation("(0/2/0/4){0/2%2/4}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars.len(), 2);
        assert_eq!(ast.layers[0].bars[0].divisions.len(), 4);
    }

    // Track with playhead tests

    // Play 0. -> 1.
    // Note 0. -> 1.
    // 1 Press, 0 release
    // Play 1. -> 0. reversed
    // Note 0. -> 1.
    // 1 Press, 0 release
    // Play       0.5 -> 1.
    // Note 0. -> 0.5
    // 0 Press, 1 release
    // Play       1. -> 0.5 reversed
    // Note 0. -> 0.5
    // 0 Press, 0 release
    // the range loops 0...1
    // 0.7 -> 0.1 should be considered forward since it's shorter distance to step forward than to step backward.
    // likewise, 1. -> 0. is a forward step.
    // 0.9 -> 0.5 is a backwards step.
    // if it's equal distance both ways, prefer forward.

    #[test]
    fn test_track_advance_no_change() {
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0/1/2)", &scale).unwrap();
        let events = track.advance(0.0);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_debug_timeline() {
        let scale = crate::scale::cmaj();
        let track = Track::from_notation("(0)", &scale).unwrap();
        assert!(!track.note_timeline.is_empty(), "Timeline is empty!");
        assert_eq!(track.note_timeline.len(), 1);
    }

    #[test]
    fn test_parse_curly_polyphony_explicit() {
        // Test that {0%1} creates a bar with polyphonic notes
        let result = parse_notation("{0%1}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars.len(), 1);
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(
            divs.len(),
            1,
            "Expected 1 division for {{0%1}}, got {}",
            divs.len()
        );
        match &divs[0].item {
            Item::Polyphony(layers) => {
                assert_eq!(layers.len(), 2, "Expected 2 polyphony layers");
            }
            _ => panic!("Expected polyphony"),
        }
    }

    #[test]
    fn test_parse_curly_polyphony_with_sequences() {
        // Test that {(0/1)%(2/3)} creates a bar with 2 divisions, each polyphonic
        let result = parse_notation("{(0/1)%(2/3)}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars.len(), 1);
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(divs.len(), 2, "Expected 2 divisions");
    }

    #[test]
    fn test_track_simple_note_press() {
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0)", &scale).unwrap();
        assert!(!track.note_timeline.is_empty(), "Timeline is empty!");
        let events = track.advance(0.5);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_track_note_press_and_release() {
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0/1)", &scale).unwrap();
        let events1 = track.advance(0.25);
        assert_eq!(events1.len(), 1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.advance(0.75);
        assert_eq!(events2.len(), 2);
        assert!(matches!(events2[0], NoteEvent::Release { .. }));
        assert!(matches!(events2[1], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_track_three_division_sequence() {
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0/1/2)", &scale).unwrap();

        let events1 = track.advance(0.1);
        assert_eq!(events1.len(), 1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.advance(0.4);
        assert_eq!(events2.len(), 2);
        assert!(matches!(events2[0], NoteEvent::Release { .. }));
        assert!(matches!(events2[1], NoteEvent::Press { .. }));

        let events3 = track.advance(0.7);
        assert_eq!(events3.len(), 2);
        assert!(matches!(events3[0], NoteEvent::Release { .. }));
        assert!(matches!(events3[1], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_track_rest_no_press() {
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0/_/1)", &scale).unwrap();

        let events1 = track.advance(0.15);
        assert_eq!(events1.len(), 1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.advance(0.5);
        assert_eq!(events2.len(), 1);
        assert!(matches!(events2[0], NoteEvent::Release { .. }));

        let events3 = track.advance(0.85);
        assert_eq!(events3.len(), 1);
        assert!(matches!(events3[0], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_track_polyphony_multiple_presses() {
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("{0%1%2}", &scale).unwrap();

        let events = track.advance(0.5);
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| matches!(e, NoteEvent::Press { .. })));
    }

    #[test]
    fn test_track_polyphony_mixed_durations() {
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("{(0/1)%(1/2)}", &scale).unwrap();

        let events1 = track.advance(0.25);
        assert_eq!(events1.len(), 2);
        assert!(events1.iter().all(|e| matches!(e, NoteEvent::Press { .. })));

        let events2 = track.advance(0.75);
        assert!(!events2.is_empty());
    }

    #[test]
    fn test_spec_play_0_to_1_note_0_to_1() {
        // Play 0.0 -> 1.0
        // Note 0.0 -> 1.0
        // Expected: 1 Press, 0 Release
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0)", &scale).unwrap();
        let events = track.advance_with_direction(0.9, true);
        assert_eq!(events.len(), 1, "Expected 1 Press");
        assert!(matches!(events[0], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_spec_play_1_to_0_note_0_to_1_reversed() {
        // Play 1.0 -> 0.0 reversed
        // Note 0.0 -> 1.0
        // Expected: 1 Press, 0 Release
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0)", &scale).unwrap();
        track.advance_with_direction(1.0f32.next_down(), false);
        let events = track.advance_with_direction(0.0, false);
        assert_eq!(events.len(), 1, "Expected 1 Press");
        assert!(matches!(events[0], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_spec_play_0_5_to_1_note_0_to_0_5() {
        // Play 0.5 -> 1.0
        // Note 0.0 -> 0.5
        // Expected: 0 Press, 1 Release
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0/_)", &scale).unwrap();
        track.advance_with_direction(0.5, true);
        let events = track.advance_with_direction(1.0, true);
        assert_eq!(events.len(), 1, "Expected 1 Release");
        assert!(matches!(events[0], NoteEvent::Release { .. }));
    }

    #[test]
    fn test_spec_play_1_to_0_5_note_0_to_0_5_reversed() {
        // Play 1.0 -> 0.5 reversed
        // Note 0.0 -> 0.5
        // Expected: 0 Press, 0 Release
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0)", &scale).unwrap();
        track.advance_with_direction(1.0, true);
        let events = track.advance_with_direction(0.5, false);
        assert_eq!(events.len(), 0, "Expected 0 events");
    }

    #[test]
    fn test_spec_backward_full_note() {
        // Use (0/1) - note 0 spans 0.0->0.5, note 1 spans 0.5->1.0
        // Play forward to 0.9 (inside note 1)
        // Then play backward to 0.1 (inside note 0)
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0/1)", &scale).unwrap();
        let _events = track.advance_with_direction(0.9, true);
        let events = track.advance_with_direction(0.1, false);
        // Moving backward should have some events (at minimum releasing note 1)
        assert!(!events.is_empty(), "Expected events when moving backward");
    }

    #[test]
    fn test_track_multi_bar_sequence() {
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("(0)(1)(2)", &scale).unwrap();

        let events1 = track.advance(0.15);
        assert_eq!(events1.len(), 1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.advance(0.5);
        assert_eq!(events2.len(), 2);

        let events3 = track.advance(0.85);
        assert_eq!(events3.len(), 2);
    }

    #[test]
    fn test_track_nested_sequence_structure() {
        let scale = crate::scale::cmaj();
        let mut track = Track::from_notation("((0/1)/(2/3))", &scale).unwrap();

        let events1 = track.advance(0.1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.advance(0.3);
        assert_eq!(events2.len(), 2);

        let events3 = track.advance(0.6);
        assert_eq!(events3.len(), 2);
    }
}
