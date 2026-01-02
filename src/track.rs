use nom::{
    Parser,
    branch::alt,
    character::complete::{char, digit1},
    combinator::opt,
    multi::{many0, many1, separated_list1},
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
    weight: usize,
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
        separated_list1(char('&'), parse_divisions),
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

fn parse_division(input: &str) -> nom::IResult<&str, (Item, usize)> {
    let (input, item) = parse_item(input)?;
    let (input, asterisks) = many0(char('*')).parse(input)?;
    let weight = 1 + asterisks.len();
    Ok((input, (item, weight)))
}

fn parse_division_with_separator(input: &str) -> nom::IResult<&str, Division> {
    let (input, (item, weight)) = parse_division(input)?;
    Ok((input, Division { item, weight }))
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
        separated_list1(char('&'), parse_one_polyphony_layer).parse(input)?;

    if poly_layers.len() == 1 {
        return Ok((input, poly_layers.into_iter().next().unwrap()));
    }

    // Create a single division containing all layers as complete sequences
    // Each layer independently subdivides the full bar time
    let result = vec![Division {
        item: Item::Polyphony(poly_layers),
        weight: 1,
    }];

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

fn strip_comments(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_comment = false;
    for c in s.chars() {
        if c == '#' {
            in_comment = !in_comment;
        } else if !in_comment {
            result.push(c);
        }
    }
    result
}

fn parse_notation(input: &str) -> Result<ParsedTrackAST, String> {
    let input = strip_comments(input);
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
            let total_weight: usize = divs.iter().map(|d| d.weight).sum();
            let mut weight_idx = 0;
            for subdiv in divs.iter() {
                let sub_start = start + (weight_idx as f32 / total_weight as f32) * div_span;
                let sub_end =
                    start + ((weight_idx + subdiv.weight) as f32 / total_weight as f32) * div_span;
                extract_notes(&subdiv.item, sub_start, sub_end, notes, scale);
                weight_idx += subdiv.weight;
            }
        }
        Item::Polyphony(layers) => {
            for layer_divs in layers {
                if layer_divs.is_empty() {
                    continue;
                }
                let div_span = end - start;
                let total_weight: usize = layer_divs.iter().map(|d| d.weight).sum();
                let mut weight_idx = 0;
                for subdiv in layer_divs.iter() {
                    let sub_start = start + (weight_idx as f32 / total_weight as f32) * div_span;
                    let sub_end = start
                        + ((weight_idx + subdiv.weight) as f32 / total_weight as f32) * div_span;
                    extract_notes(&subdiv.item, sub_start, sub_end, notes, scale);
                    weight_idx += subdiv.weight;
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
    bar_count: usize,
}

impl Track {
    pub fn parse(notation: &str, scale: &crate::Scale) -> Result<Self, String> {
        let ast = parse_notation(notation)?;
        let mut events = Vec::new();

        let bar_count = ast.layers.iter().map(|l| l.bars.len()).max().unwrap_or(1);

        for layer in &ast.layers {
            let num_bars = layer.bars.len();
            let mut bar_idx = 0;

            for bar in &layer.bars {
                let bar_start = bar_idx as f32 / num_bars as f32;
                let bar_end = (bar_idx + 1) as f32 / num_bars as f32;
                let bar_span = bar_end - bar_start;

                let total_weight: usize = bar.divisions.iter().map(|d| d.weight).sum();
                let mut weight_idx = 0;

                for division in &bar.divisions {
                    let div_start = bar_start + (weight_idx as f32 / total_weight as f32) * bar_span;
                    let div_end = bar_start + ((weight_idx + division.weight) as f32 / total_weight as f32) * bar_span;
                    extract_notes(&division.item, div_start, div_end, &mut events, scale);
                    weight_idx += division.weight;
                }
                bar_idx += 1;
            }
        }

        events.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());

        Ok(Track {
            playhead: 0.0,
            note_timeline: events,
            bar_count,
        })
    }

    pub fn bar_count(&self) -> usize {
        self.bar_count
    }

    pub fn play(&mut self, to: f32) -> Vec<NoteEvent> {
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
    fn test_parse_weight_single_asterisk() {
        let result = parse_notation("(0*/1)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions[0].weight, 2);
        assert_eq!(ast.layers[0].bars[0].divisions[1].weight, 1);
    }

    #[test]
    fn test_parse_weight_multiple_asterisks() {
        let result = parse_notation("(0**/1)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions[0].weight, 3);
        assert_eq!(ast.layers[0].bars[0].divisions[1].weight, 1);
    }

    #[test]
    fn test_parse_weight_with_chromatic_shift() {
        let result = parse_notation("(0+**/1)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions[0].weight, 3);
        match &ast.layers[0].bars[0].divisions[0].item {
            Item::Note(n) => {
                assert_eq!(n.degree, 0);
                assert_eq!(n.chromatic_shift, 1);
            }
            _ => panic!("Expected note with chromatic shift"),
        }
    }

    #[test]
    fn test_parse_rest_with_weight() {
        let result = parse_notation("(_***/0)");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars[0].divisions[0].weight, 4);
        assert!(matches!(
            ast.layers[0].bars[0].divisions[0].item,
            Item::Rest
        ));
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
        let result = parse_notation("{(0/1)&(2/3)}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars.len(), 1);
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(divs.len(), 1, "Should be 1 polyphonic division");
        match &divs[0].item {
            Item::Polyphony(layers) => {
                assert_eq!(layers.len(), 2, "Should have 2 layers");
                assert_eq!(layers[0].len(), 2, "First layer has 2 divisions (0/1)");
                assert_eq!(layers[1].len(), 2, "Second layer has 2 divisions (2/3)");
            }
            _ => panic!("Expected polyphony"),
        }
    }

    #[test]
    fn test_parse_polyphony_mismatched_divisions() {
        let result = parse_notation("{(0/2/4)&(1/3)}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(
            divs.len(),
            1,
            "Should be 1 polyphonic division with independent layers"
        );
        match &divs[0].item {
            Item::Polyphony(layers) => {
                assert_eq!(layers.len(), 2, "Should have 2 layers");
                assert_eq!(layers[0].len(), 3, "First layer has 3 divisions");
                assert_eq!(layers[1].len(), 2, "Second layer has 2 divisions");
            }
            _ => panic!("Expected polyphony"),
        }
    }

    #[test]
    fn test_parse_nested_polyphony() {
        let result = parse_notation("(({0&2}/{1&3})/4)");
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
        let result = parse_notation("(0/2/0/4){0/2&2/4}");
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
        let mut track = Track::parse("(0/1/2)", &scale).unwrap();
        let events = track.play(0.0);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_debug_timeline() {
        let scale = crate::scale::cmaj();
        let track = Track::parse("(0)", &scale).unwrap();
        assert!(!track.note_timeline.is_empty(), "Timeline is empty!");
        assert_eq!(track.note_timeline.len(), 1);
    }

    #[test]
    fn test_parse_curly_polyphony_explicit() {
        // Test that {0&1} creates a bar with polyphonic notes
        let result = parse_notation("{0&1}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars.len(), 1);
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(
            divs.len(),
            1,
            "Expected 1 division for {{0&1}}, got {}",
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
        // Test that {(0/1)&(2/3)} creates a bar with 1 polyphonic division containing 2 layers
        let result = parse_notation("{(0/1)&(2/3)}");
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.layers[0].bars.len(), 1);
        let divs = &ast.layers[0].bars[0].divisions;
        assert_eq!(divs.len(), 1, "Expected 1 polyphonic division");
        match &divs[0].item {
            Item::Polyphony(layers) => {
                assert_eq!(layers.len(), 2);
                assert_eq!(layers[0].len(), 2);
                assert_eq!(layers[1].len(), 2);
            }
            _ => panic!("Expected polyphony"),
        }
    }

    #[test]
    fn test_track_simple_note_press() {
        let scale = crate::scale::cmaj();
        let mut track = Track::parse("(0)", &scale).unwrap();
        assert!(!track.note_timeline.is_empty(), "Timeline is empty!");
        let events = track.play(0.5);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_track_note_press_and_release() {
        let scale = crate::scale::cmaj();
        let mut track = Track::parse("(0/1)", &scale).unwrap();
        let events1 = track.play(0.25);
        assert_eq!(events1.len(), 1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.play(0.75);
        assert_eq!(events2.len(), 2);
        assert!(matches!(events2[0], NoteEvent::Release { .. }));
        assert!(matches!(events2[1], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_track_three_division_sequence() {
        let scale = crate::scale::cmaj();
        let mut track = Track::parse("(0/1/2)", &scale).unwrap();

        let events1 = track.play(0.1);
        assert_eq!(events1.len(), 1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.play(0.4);
        assert_eq!(events2.len(), 2);
        assert!(matches!(events2[0], NoteEvent::Release { .. }));
        assert!(matches!(events2[1], NoteEvent::Press { .. }));

        let events3 = track.play(0.7);
        assert_eq!(events3.len(), 2);
        assert!(matches!(events3[0], NoteEvent::Release { .. }));
        assert!(matches!(events3[1], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_track_rest_no_press() {
        let scale = crate::scale::cmaj();
        let mut track = Track::parse("(0/_/1)", &scale).unwrap();

        let events1 = track.play(0.15);
        assert_eq!(events1.len(), 1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.play(0.5);
        assert_eq!(events2.len(), 1);
        assert!(matches!(events2[0], NoteEvent::Release { .. }));

        let events3 = track.play(0.85);
        assert_eq!(events3.len(), 1);
        assert!(matches!(events3[0], NoteEvent::Press { .. }));
    }

    #[test]
    fn test_track_polyphony_multiple_presses() {
        let scale = crate::scale::cmaj();
        let mut track = Track::parse("{0&1&2}", &scale).unwrap();

        let events = track.play(0.5);
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| matches!(e, NoteEvent::Press { .. })));
    }

    #[test]
    fn test_track_polyphony_mixed_durations() {
        let scale = crate::scale::cmaj();
        let mut track = Track::parse("{(0/1)&(1/2)}", &scale).unwrap();

        let events1 = track.play(0.25);
        assert_eq!(events1.len(), 2);
        assert!(events1.iter().all(|e| matches!(e, NoteEvent::Press { .. })));

        let events2 = track.play(0.75);
        assert!(!events2.is_empty());
    }

    #[test]
    fn test_spec_play_0_to_1_note_0_to_1() {
        // Play 0.0 -> 1.0
        // Note 0.0 -> 1.0
        // Expected: 1 Press, 0 Release
        let scale = crate::scale::cmaj();
        let mut track = Track::parse("(0)", &scale).unwrap();
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
        let mut track = Track::parse("(0)", &scale).unwrap();
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
        let mut track = Track::parse("(0/_)", &scale).unwrap();
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
        let mut track = Track::parse("(0)", &scale).unwrap();
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
        let mut track = Track::parse("(0/1)", &scale).unwrap();
        let _events = track.advance_with_direction(0.9, true);
        let events = track.advance_with_direction(0.1, false);
        // Moving backward should have some events (at minimum releasing note 1)
        assert!(!events.is_empty(), "Expected events when moving backward");
    }

    #[test]
    fn test_track_multi_bar_sequence() {
        let scale = crate::scale::cmaj();
        let mut track = Track::parse("(0)(1)(2)", &scale).unwrap();

        let events1 = track.play(0.15);
        assert_eq!(events1.len(), 1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.play(0.5);
        assert_eq!(events2.len(), 2);

        let events3 = track.play(0.85);
        assert_eq!(events3.len(), 2);
    }

    #[test]
    fn test_track_nested_sequence_structure() {
        let scale = crate::scale::cmaj();
        let mut track = Track::parse("((0/1)/(2/3))", &scale).unwrap();

        let events1 = track.play(0.1);
        assert!(matches!(events1[0], NoteEvent::Press { .. }));

        let events2 = track.play(0.3);
        assert_eq!(events2.len(), 2);

        let events3 = track.play(0.6);
        assert_eq!(events3.len(), 2);
    }

    #[test]
    fn test_polyrhythm_different_divisions() {
        let notation = r#"{
            {0&3&5}/{1&3&5&7}/{2&4&6}/{1&3&6}
            &
            (12/13/14/(15/16/15/14)/12)
        }"#;

        let result = parse_notation(notation);
        assert!(result.is_ok(), "Should parse polyrhythm notation");

        let ast = result.unwrap();
        assert_eq!(ast.layers.len(), 1);
        assert_eq!(ast.layers[0].bars.len(), 1);

        let divisions = &ast.layers[0].bars[0].divisions;

        // Should be a single division containing polyphony with 2 complete layers
        assert_eq!(divisions.len(), 1, "Should be 1 polyphonic division");

        match &divisions[0].item {
            Item::Polyphony(layers) => {
                assert_eq!(layers.len(), 2, "Should have 2 layers");

                // First layer: {0&3&5}/{1&3&5&7}/{2&4&6}/{1&3&6} = 4 divisions
                assert_eq!(layers[0].len(), 4, "First layer should have 4 divisions");

                // Second layer: (12/13/14/(15/16/15/14)/12) = 5 divisions
                assert_eq!(layers[1].len(), 5, "Second layer should have 5 divisions");

                // Each layer independently subdivides the same time span
                // creating a polyrhythm
            }
            _ => panic!("Expected polyphony"),
        }
    }
}
