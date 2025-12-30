# Track Notation Spec

## Basic Structure

- **Bars**: `(...)` Top-level parentheses define bars, executed sequentially
- **Divisions**: Subdivide bar time by weight. `/` separates divisions within a level
- **Rest**: `_` silence for one division
- **Weight**: `*` after an item increases its weight (time allocation) by 1 unit per asterisk
  - `(0/1)` = equal split (1/2, 1/2)
  - `(0*/1)` = 0 gets 2 units, 1 gets 1 unit (2/3, 1/3)
  - `(0**/1)` = 0 gets 3 units, 1 gets 1 unit (3/4, 1/4)
- **Nesting**: `((...)(...))` subdivide divisions further

## Note Specification

- `0`, `1`, `2...`: Scale degree (0-indexed, wraps with octaves)
- `23`: Scale degree 23 (multi-digit number, no slash)
- `-1`, `-23`: Negative scale degree (descending, wraps correctly through octaves)
- `0+`, `2-`: Chromatic shift (+ for sharp/up semitone, - for flat/down semitone)
- `0*`, `0**`, etc.: Weight modifiers (asterisks come after chromatic shifts)
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
| `(0*/1)` | Degree 0 takes 2/3 of bar, degree 1 takes 1/3 |
| `(0**/1)` | Degree 0 takes 3/4 of bar, degree 1 takes 1/4 |
| `(_***/0)` | Rest for 4/5 of bar, degree 0 for 1/5 |
| `((0/_/1/2)/(0/_/1))` | First half: 0,rest,1,2. Second half: 0,rest,1 |
| `(0)(1)` | Two bars: degree 0, then degree 1 |
| `{(0/1)%(2/3)}` | Two simultaneous layers |
| `(-1/1/_/2)` | Degree -1, degree 1, rest, degree 2 |
| `(0+/2-/4)` | Degree 0 sharp, degree 2 flat, degree 4 natural |
| `(0+**/1)` | Degree 0 sharp takes 3/4, degree 1 takes 1/4 |
