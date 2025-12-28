# Track Notation Spec

## Basic Structure

- **Bars**: `(...)` Top-level parentheses define bars, executed sequentially
- **Divisions**: Subdivide bar time equally. `/` separates divisions within a level
- **Rest**: `_` silence for one division
- **Legato**: `~` connects notes without release, creating slides/bends
- **Nudge**: `<N>` on separator adjusts transition timing (0-100, % of duration)
  - Before separator: `<30>/` nudges end of preceding note 30% later, start of following note 30% later
  - After separator: `/<10>` nudges end of preceding note 10% earlier, start of following note 10% earlier
- **Nesting**: `((...)(...))` subdivide divisions further

## Note Specification

- `0`, `1`, `2...`: Scale degree (0-indexed, wraps with octaves)
- `23`: Scale degree 23 (multi-digit number, no slash)
- `-1`, `-23`: Negative scale degree (descending, wraps correctly through octaves)
- `0+`, `2-`: Chromatic shift (+ for sharp/up semitone, - for flat/down semitone)
- `/` separates all divisions: notes, rests, and nested sections

## Polyphony

- `{(...) % (...)}`: Simultaneous layers (layers separated by `%`)
- Each layer is a complete bar/section
- All active layers output together

## Examples

| Notation | Behavior |
|----------|----------|
| `(0)` | Play scale degree 0 for entire bar |
| `(0/_/2)` | Degree 0, rest, degree 2 (three equal divisions) |
| `(0/1/2)` | Three divisions: degrees 0, 1, 2 (release between each) |
| `(0~1~2)` | Degrees 0→1→2 legato (no release, continuous bend) |
| `(0/<30>1<10>/2)` | Note 0 ends 30% late, note 1 starts 30% late and ends 10% early, note 2 starts 10% early |
| `((0/_/1/2)/(0/_/1))` | First half: 0,rest,1,2. Second half: 0,rest,1 |
| `(0)(1)` | Two bars: degree 0, then degree 1 |
| `{(0/1)%(2/3)}` | Two simultaneous layers |
| `(-1/1/_/2)` | Degree -1, degree 1, rest, degree 2 |
| `(0+/2-/4)` | Degree 0 sharp, degree 2 flat, degree 4 natural |
