use super::{Sequence, Stroke};
use crate::scale::Scale;
use nom::{
    Parser,
    branch::alt,
    character::complete::{char, digit1},
    combinator::{all_consuming, map_res, opt, recognize},
    multi::{many0_count, many1, separated_list1},
    sequence::delimited,
};

enum Item {
    Note { degree: i32, shift: i32 },
    Rest,
    Sequence(Vec<Division>),
    Polyphony(Vec<Vec<Division>>),
}

struct Division {
    item: Item,
    weight: usize,
}

fn group(input: &str) -> nom::IResult<&str, Item> {
    alt((
        delimited(char('('), divisions, char(')')).map(Item::Sequence),
        delimited(char('{'), separated_list1(char('&'), divisions), char('}')).map(Item::Polyphony),
    ))
    .parse(input)
}

fn divisions(input: &str) -> nom::IResult<&str, Vec<Division>> {
    separated_list1(
        char('/'),
        (
            alt((
                group,
                (
                    map_res(recognize((opt(char('-')), digit1)), str::parse::<i32>),
                    opt(alt((char('+').map(|_| 1), char('-').map(|_| -1)))),
                )
                    .map(|(degree, shift)| Item::Note {
                        degree,
                        shift: shift.unwrap_or(0),
                    }),
                char('_').map(|_| Item::Rest),
            )),
            many0_count(char('*')),
        )
            .map(|(item, stars)| Division {
                item,
                weight: stars + 1,
            }),
    )
    .parse(input)
}

fn emit(
    item: &Item,
    start: f32,
    end: f32,
    strokes: &mut Vec<Stroke>,
    scale: &Scale,
) -> Result<(), &'static str> {
    match item {
        Item::Note { degree, shift } => {
            let pitch = (scale.note(degree.rem_euclid(7)) as i64
                + degree.div_euclid(7) as i64 * 12
                + *shift as i64)
                .clamp(0, 127) as f32;
            strokes.push(Stroke::try_from(vec![
                [start, pitch, 1.0],
                [end, pitch, 1.0],
            ])?);
        }
        Item::Rest => {}
        Item::Sequence(divisions) => divide(divisions, start, end, strokes, scale)?,
        Item::Polyphony(layers) => {
            for divisions in layers {
                divide(divisions, start, end, strokes, scale)?;
            }
        }
    }
    Ok(())
}

fn divide(
    divisions: &[Division],
    start: f32,
    end: f32,
    strokes: &mut Vec<Stroke>,
    scale: &Scale,
) -> Result<(), &'static str> {
    let total: usize = divisions.iter().map(|division| division.weight).sum();
    let mut weight = 0;
    for division in divisions {
        let from = start + weight as f32 / total as f32 * (end - start);
        weight += division.weight;
        let to = start + weight as f32 / total as f32 * (end - start);
        emit(&division.item, from, to, strokes, scale)?;
    }
    Ok(())
}

pub fn parse_sequence(notation: &str, scale: &Scale) -> Result<Sequence, String> {
    let input: String = notation
        .lines()
        .flat_map(|line| line.split('#').next().unwrap().chars())
        .filter(|c| !c.is_whitespace())
        .collect();
    let (_, groups) = all_consuming(many1(group))
        .parse(&input)
        .map_err(|error| format!("Parse error: {error}"))?;
    let bars = u16::try_from(groups.len()).map_err(|_| "Too many bars")?;
    let mut strokes = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        emit(group, index as f32, (index + 1) as f32, &mut strokes, scale)?;
    }
    Sequence::new(strokes, bars).map_err(str::to_string)
}
