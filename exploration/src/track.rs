use nom::{
    IResult,
    branch::alt,
    character::complete::{char, digit1},
    combinator::{map, opt},
    multi::{many0, many1, separated_list1},
    sequence::{delimited, tuple},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub degree: i32,
    pub chromatic_shift: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Note(Note),
    Rest,
    Sequence(Vec<Division>),
    Polyphony(Vec<Vec<Division>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Division {
    pub item: Item,
    pub weight: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub divisions: Vec<Division>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub bars: Vec<Bar>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub layers: Vec<Layer>,
}

fn parse_number(input: &str) -> IResult<&str, i32> {
    map(
        tuple((opt(char('-')), digit1)),
        |(sign, num): (Option<char>, &str)| {
            let val: i32 = num.parse().unwrap();
            if sign.is_some() { -val } else { val }
        },
    )(input)
}



fn parse_note(input: &str) -> IResult<&str, Item> {
    let (input, degree) = parse_number(input)?;
    let (input, chromatic_shift) =
        opt(alt((map(char('+'), |_| 1i32), map(char('-'), |_| -1i32))))(input)?;
    Ok((
        input,
        Item::Note(Note {
            degree,
            chromatic_shift: chromatic_shift.unwrap_or(0),
        }),
    ))
}

fn parse_rest(input: &str) -> IResult<&str, Item> {
    map(char('_'), |_| Item::Rest)(input)
}

fn parse_nested_bar(input: &str) -> IResult<&str, Item> {
    let (input, divisions) = delimited(char('('), parse_divisions, char(')'))(input)?;
    Ok((input, Item::Sequence(divisions)))
}

fn parse_polyphonic_item(input: &str) -> IResult<&str, Item> {
    delimited(
        char('{'),
        map(separated_list1(char('%'), parse_divisions), Item::Polyphony),
        char('}'),
    )(input)
}

fn parse_item(input: &str) -> IResult<&str, Item> {
    alt((
        parse_polyphonic_item,
        parse_nested_bar,
        parse_note,
        parse_rest,
    ))(input)
}

fn parse_division(input: &str) -> IResult<&str, (Item, usize)> {
    let (input, item) = parse_item(input)?;
    let (input, asterisks) = many0(char('*'))(input)?;
    let weight = 1 + asterisks.len();
    Ok((input, (item, weight)))
}

fn parse_division_with_separator(input: &str) -> IResult<&str, Division> {
    let (input, (item, weight)) = parse_division(input)?;
    Ok((input, Division { item, weight }))
}

fn parse_divisions(input: &str) -> IResult<&str, Vec<Division>> {
    separated_list1(char('/'), parse_division_with_separator)(input)
}

fn parse_polyphonic_divisions(input: &str) -> IResult<&str, Vec<Division>> {
    map(separated_list1(char('%'), parse_divisions), |poly_layers| {
        // If only one layer, return divisions as-is (no polyphony)
        if poly_layers.len() == 1 {
            return poly_layers.into_iter().next().unwrap();
        }

        // Flatten into a single division list where each is polyphony
        // {0/2%2/4} should become: [Polyphony([[0], [2]]), Polyphony([[2], [4]])]
        let max_divs = poly_layers.iter().map(|l| l.len()).max().unwrap_or(0);
        (0..max_divs)
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
                    weight: 1,
                }
            })
            .collect()
    })(input)
}

fn parse_bar(input: &str) -> IResult<&str, Bar> {
    let (input, divisions) = alt((
        delimited(char('('), parse_divisions, char(')')),
        delimited(char('{'), parse_polyphonic_divisions, char('}')),
    ))(input)?;
    Ok((input, Bar { divisions }))
}

fn parse_layer_sequence(input: &str) -> IResult<&str, Layer> {
    let (input, bars) = many1(parse_bar)(input)?;
    Ok((input, Layer { bars }))
}

fn parse_track_section(input: &str) -> IResult<&str, Vec<Layer>> {
    map(parse_layer_sequence, |layer| vec![layer])(input)
}

fn parse_track(input: &str) -> IResult<&str, Track> {
    let (input, sections) = many1(parse_track_section)(input)?;
    let layers = sections.into_iter().flatten().collect();
    Ok((input, Track { layers }))
}

pub fn parse(input: &str) -> Result<Track, String> {
    let input: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    match parse_track(&input) {
        Ok((_, track)) => Ok(track),
        Err(e) => Err(format!("Parse error: {:?}", e)),
    }
}

#[derive(Clone, Debug)]
pub struct NoteEvent {
    pub pitch: u8,
    pub start: f32,
    pub end: f32,
}

#[derive(Clone, Debug)]
pub struct ParsedTrack {
    pub notes: Vec<NoteEvent>,
    cached_clock_pos: Option<f32>,
    cached_active_pitches: Vec<u8>,
}

impl ParsedTrack {
    pub fn query(&mut self, key: crate::Key, clock_position: f32) -> crate::instrument::PressState {
        // Clamp to 0.0-1.0
        let pos = clock_position.clamp(0.0, 1.0);

        // Check cache and update if needed
        if self.cached_clock_pos != Some(pos) {
            self.cached_active_pitches.clear();
            
            // Binary search to find notes that could be active
            let idx = self.notes.partition_point(|note| note.start <= pos);
            
            // Check notes starting before or at pos
            for i in (0..idx).rev() {
                if self.notes[i].end > pos {
                    if !self.cached_active_pitches.contains(&self.notes[i].pitch) {
                        self.cached_active_pitches.push(self.notes[i].pitch);
                    }
                } else {
                    break; // Notes are sorted, no more active notes backwards
                }
            }
            
            // Check notes starting at or after pos (if they could be active at exactly pos)
            for i in idx..self.notes.len() {
                if self.notes[i].start <= pos && self.notes[i].end > pos {
                    if !self.cached_active_pitches.contains(&self.notes[i].pitch) {
                        self.cached_active_pitches.push(self.notes[i].pitch);
                    }
                } else if self.notes[i].start > pos {
                    break; // No more notes can be active
                }
            }
            
            self.cached_clock_pos = Some(pos);
        }

        // Each voice checks if it's active at this position
        if self.cached_active_pitches.contains(&key.pitch) {
            crate::instrument::PressState::Pressed {
                pressed_at: 0,
                time_in_state: 0,
            }
        } else {
            crate::instrument::PressState::Idle
        }
    }

    pub fn from_track(track: &Track, scale: &crate::Scale) -> Self {
        let mut notes = Vec::new();
        let mut total_weight = 0;

        // First pass: sum total weight across all divisions
        for layer in &track.layers {
            for bar in &layer.bars {
                for division in &bar.divisions {
                    total_weight += division.weight;
                }
            }
        }

        let mut weight_idx = 0;

        // Second pass: extract notes with percentage positions based on weight
        for layer in &track.layers {
            for bar in &layer.bars {
                for division in &bar.divisions {
                    let div_start = weight_idx as f32 / total_weight as f32;
                    let div_end = (weight_idx + division.weight) as f32 / total_weight as f32;

                    Self::extract_notes(&division.item, div_start, div_end, &mut notes, scale);
                    weight_idx += division.weight;
                }
            }
        }

        // Sort notes by start position for binary search
        notes.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());

        ParsedTrack {
            notes,
            cached_clock_pos: None,
            cached_active_pitches: Vec::new(),
        }
    }

    fn extract_notes(
        item: &Item,
        start: f32,
        end: f32,
        notes: &mut Vec<NoteEvent>,
        scale: &crate::Scale,
    ) {
        match item {
            Item::Note(note) => {
                let pitch = (scale.degree(note.degree) + note.chromatic_shift).clamp(0, 127) as u8;
                notes.push(NoteEvent { pitch, start, end });
            }
            Item::Rest => {
                // Rests add no notes
            }
            Item::Sequence(divs) => {
                let div_span = end - start;
                let total_weight: usize = divs.iter().map(|d| d.weight).sum();
                let mut weight_idx = 0;
                for subdiv in divs.iter() {
                    let sub_start = start + (weight_idx as f32 / total_weight as f32) * div_span;
                    let sub_end = start + ((weight_idx + subdiv.weight) as f32 / total_weight as f32) * div_span;
                    Self::extract_notes(&subdiv.item, sub_start, sub_end, notes, scale);
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
                        let sub_end = start + ((weight_idx + subdiv.weight) as f32 / total_weight as f32) * div_span;
                        Self::extract_notes(&subdiv.item, sub_start, sub_end, notes, scale);
                        weight_idx += subdiv.weight;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(degree: i32, shift: i32) -> Item {
        Item::Note(Note {
            degree,
            chromatic_shift: shift,
        })
    }

    #[test]
    fn test_parse_note_single_digit() {
        assert_eq!(parse_note("0"), Ok(("", note(0, 0))));
        assert_eq!(parse_note("5"), Ok(("", note(5, 0))));
    }

    #[test]
    fn test_parse_note_multi_digit() {
        assert_eq!(parse_note("23"), Ok(("", note(23, 0))));
        assert_eq!(parse_note("100"), Ok(("", note(100, 0))));
    }

    #[test]
    fn test_parse_note_negative() {
        assert_eq!(parse_note("-1"), Ok(("", note(-1, 0))));
        assert_eq!(parse_note("-23"), Ok(("", note(-23, 0))));
    }

    #[test]
    fn test_parse_note_with_chromatic_shift() {
        assert_eq!(parse_note("0+"), Ok(("", note(0, 1))));
        assert_eq!(parse_note("2-"), Ok(("", note(2, -1))));
    }

    #[test]
    fn test_parse_rest() {
        assert_eq!(parse_rest("_"), Ok(("", Item::Rest)));
    }

    #[test]
    fn test_parse_weight_single_asterisk() {
        let result = parse("(0*/1)").unwrap();
        assert_eq!(result.layers[0].bars[0].divisions[0].weight, 2);
        assert_eq!(result.layers[0].bars[0].divisions[1].weight, 1);
    }

    #[test]
    fn test_parse_weight_multiple_asterisks() {
        let result = parse("(0**/1)").unwrap();
        assert_eq!(result.layers[0].bars[0].divisions[0].weight, 3);
        assert_eq!(result.layers[0].bars[0].divisions[1].weight, 1);
    }

    #[test]
    fn test_parse_weight_with_chromatic_shift() {
        let result = parse("(0+**/1)").unwrap();
        assert_eq!(result.layers[0].bars[0].divisions[0].weight, 3);
        assert!(matches!(
            result.layers[0].bars[0].divisions[0].item,
            Item::Note(Note {
                degree: 0,
                chromatic_shift: 1
            })
        ));
    }

    #[test]
    fn test_parse_simple_bar() {
        let result = parse("(0/1/2)").unwrap();
        assert_eq!(result.layers.len(), 1);
        assert_eq!(result.layers[0].bars.len(), 1);
        assert_eq!(result.layers[0].bars[0].divisions.len(), 3);
    }

    #[test]
    fn test_parse_bar_with_rest() {
        let result = parse("(0/_/2)").unwrap();
        assert_eq!(result.layers[0].bars[0].divisions.len(), 3);
        assert!(matches!(
            result.layers[0].bars[0].divisions[0].item,
            Item::Note(Note {
                degree: 0,
                chromatic_shift: 0
            })
        ));
        assert!(matches!(
            result.layers[0].bars[0].divisions[1].item,
            Item::Rest
        ));
        assert!(matches!(
            result.layers[0].bars[0].divisions[2].item,
            Item::Note(Note {
                degree: 2,
                chromatic_shift: 0
            })
        ));
    }

    #[test]
    fn test_parse_single_note_bar() {
        let result = parse("(0)").unwrap();
        assert_eq!(result.layers[0].bars.len(), 1);
        assert_eq!(result.layers[0].bars[0].divisions.len(), 1);
    }

    #[test]
    fn test_parse_multiple_bars() {
        let result = parse("(0)(1)(2)").unwrap();
        assert_eq!(result.layers[0].bars.len(), 3);
        assert_eq!(result.layers[0].bars[0].divisions.len(), 1);
        assert_eq!(result.layers[0].bars[1].divisions.len(), 1);
        assert_eq!(result.layers[0].bars[2].divisions.len(), 1);
    }



    #[test]
    fn test_parse_negative_degrees() {
        let result = parse("(-1/0/1)").unwrap();
        assert!(matches!(
            result.layers[0].bars[0].divisions[0].item,
            Item::Note(Note {
                degree: -1,
                chromatic_shift: 0
            })
        ));
        assert!(matches!(
            result.layers[0].bars[0].divisions[1].item,
            Item::Note(Note {
                degree: 0,
                chromatic_shift: 0
            })
        ));
        assert!(matches!(
            result.layers[0].bars[0].divisions[2].item,
            Item::Note(Note {
                degree: 1,
                chromatic_shift: 0
            })
        ));
    }

    #[test]
    fn test_parse_complex_pattern_with_weights() {
        let result = parse("(0*/1**/2)").unwrap();
        assert_eq!(result.layers[0].bars[0].divisions.len(), 3);

        let div0 = &result.layers[0].bars[0].divisions[0];
        assert!(matches!(
            div0.item,
            Item::Note(Note {
                degree: 0,
                chromatic_shift: 0
            })
        ));
        assert_eq!(div0.weight, 2);

        let div1 = &result.layers[0].bars[0].divisions[1];
        assert!(matches!(
            div1.item,
            Item::Note(Note {
                degree: 1,
                chromatic_shift: 0
            })
        ));
        assert_eq!(div1.weight, 3);

        let div2 = &result.layers[0].bars[0].divisions[2];
        assert!(matches!(
            div2.item,
            Item::Note(Note {
                degree: 2,
                chromatic_shift: 0
            })
        ));
        assert_eq!(div2.weight, 1);
    }

    #[test]
    fn test_parse_rest_in_complex_pattern() {
        let result = parse("(-1/1/_/2)").unwrap();
        assert_eq!(result.layers[0].bars[0].divisions.len(), 4);
        assert!(matches!(
            result.layers[0].bars[0].divisions[2].item,
            Item::Rest
        ));
    }

    #[test]
    fn test_parse_sequence_pattern() {
        let result = parse("(0/1/2)(3/4/5)").unwrap();
        assert_eq!(result.layers[0].bars.len(), 2);
        assert_eq!(result.layers[0].bars[0].divisions.len(), 3);
        assert_eq!(result.layers[0].bars[1].divisions.len(), 3);
    }

    #[test]
    fn test_parse_all_single_digits() {
        for i in 0..10 {
            let input = format!("({})", i);
            let result = parse(&input).unwrap();
            assert!(
                matches!(result.layers[0].bars[0].divisions[0].item, Item::Note(Note { degree, chromatic_shift: 0 }) if degree == i)
            );
        }
    }

    #[test]
    fn test_parse_rest_with_weight() {
        let result = parse("(_***/0)").unwrap();
        assert_eq!(result.layers[0].bars[0].divisions[0].weight, 4);
        assert!(matches!(
            result.layers[0].bars[0].divisions[0].item,
            Item::Rest
        ));
    }

    #[test]
    fn test_parse_single_polyphonic_layer() {
        let result = parse("{(0/1)}").unwrap();
        assert_eq!(result.layers.len(), 1);
        assert_eq!(result.layers[0].bars.len(), 1);
    }

    #[test]
    fn test_parse_polyphony_inside_divisions() {
        let result = parse("({0%2%4}/{1%3%5})").unwrap();
        assert_eq!(result.layers.len(), 1);
        assert_eq!(result.layers[0].bars[0].divisions.len(), 2);

        // First division should be a polyphony with 3 layers
        match &result.layers[0].bars[0].divisions[0].item {
            Item::Polyphony(layers) => {
                assert_eq!(layers.len(), 3);
                assert_eq!(layers[0].len(), 1); // {0}
                assert_eq!(layers[1].len(), 1); // {2}
                assert_eq!(layers[2].len(), 1); // {4}
            }
            _ => panic!("Expected polyphony in first division"),
        }

        // Second division should be a polyphony with 3 layers
        match &result.layers[0].bars[0].divisions[1].item {
            Item::Polyphony(layers) => {
                assert_eq!(layers.len(), 3);
                assert_eq!(layers[0].len(), 1); // {1}
                assert_eq!(layers[1].len(), 1); // {3}
                assert_eq!(layers[2].len(), 1); // {5}
            }
            _ => panic!("Expected polyphony in second division"),
        }
    }

    #[test]
    fn test_parse_nested_polyphony_complex() {
        let result = parse("(({0%2}/{1%3})/4)").unwrap();
        assert_eq!(result.layers.len(), 1);
        assert_eq!(result.layers[0].bars[0].divisions.len(), 2);

        // First division is a sequence with polyphony inside
        match &result.layers[0].bars[0].divisions[0].item {
            Item::Sequence(divs) => {
                assert_eq!(divs.len(), 2);
                // First subdiv should be polyphony
                match &divs[0].item {
                    Item::Polyphony(layers) => {
                        assert_eq!(layers.len(), 2);
                    }
                    _ => panic!("Expected polyphony in first subdiv"),
                }
            }
            _ => panic!("Expected sequence in first division"),
        }
    }

    #[test]
    fn test_parse_curly_braces_as_bar() {
        let result = parse("{0/2/4}").unwrap();
        assert_eq!(result.layers.len(), 1);
        assert_eq!(result.layers[0].bars.len(), 1);
        assert_eq!(result.layers[0].bars[0].divisions.len(), 3);
        assert_eq!(result.layers[0].bars[0].divisions[0].item, note(0, 0));
        assert_eq!(result.layers[0].bars[0].divisions[1].item, note(2, 0));
        assert_eq!(result.layers[0].bars[0].divisions[2].item, note(4, 0));
    }

    #[test]
    fn test_sequential_bars_with_polyphony_in_second() {
        let result = parse("(0/2/0/4){0/2%2/4}").unwrap();

        eprintln!("Layers: {}", result.layers.len());
        for (l, layer) in result.layers.iter().enumerate() {
            eprintln!("  Layer {}: {} bars", l, layer.bars.len());
            for (b, bar) in layer.bars.iter().enumerate() {
                eprintln!("    Bar {}: {} divisions", b, bar.divisions.len());
            }
        }

        // Should have 1 layer with 2 sequential bars
        assert_eq!(result.layers.len(), 1);
        assert_eq!(result.layers[0].bars.len(), 2);

        // First bar: (0/2/0/4) - 4 divisions, all monophonic notes
        assert_eq!(result.layers[0].bars[0].divisions.len(), 4);
        assert!(matches!(
            result.layers[0].bars[0].divisions[0].item,
            Item::Note(Note {
                degree: 0,
                chromatic_shift: 0
            })
        ));
        assert!(matches!(
            result.layers[0].bars[0].divisions[1].item,
            Item::Note(Note {
                degree: 2,
                chromatic_shift: 0
            })
        ));
        assert!(matches!(
            result.layers[0].bars[0].divisions[2].item,
            Item::Note(Note {
                degree: 0,
                chromatic_shift: 0
            })
        ));
        assert!(matches!(
            result.layers[0].bars[0].divisions[3].item,
            Item::Note(Note {
                degree: 4,
                chromatic_shift: 0
            })
        ));

        // Second bar: {0/2%2/4} - structure depends on how % is parsed
        eprintln!(
            "Second bar divisions: {}",
            result.layers[0].bars[1].divisions.len()
        );
        for (d, div) in result.layers[0].bars[1].divisions.iter().enumerate() {
            eprintln!("  Division {}: {:?}", d, div.item);
        }
    }
}
