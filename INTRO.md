# brainwash

A real-time audio synthesis library built in Rust with allocation-free audio processing.

## Architecture

brainwash is designed around composable audio units:

- **Oscillators** - sine, square, triangle, saw, noise waveforms
- **Envelopes** - ADSR with configurable attack/decay/sustain/release
- **Filters** - lowpass, highpass, bandpass filtering
- **Effects** - reverb, flanger, distortion
- **Track system** - notation-based sequencing with scale support
- **Keyboard** - polyphonic voice management with per-key state

Each unit follows strict design principles: minimal API, injected dependencies, and airtight models that minimize invalid states.

## Features

- `live` - real-time audio output via cpal (default)
- `wav` - WAV file export via hound
- `tui` - terminal UI support

## Usage

```rust
use brainwash::*;

let mut clock = Clock::default().bpm(120.).bars(4.);
let scale = cmin();
let mut osc = Osc::default();

play_live(move |s| {
    osc.sin().freq(440.).output(s)
}).expect("Error with live audio");
```

See `examples/simple` for a basic example and `examples/coin` for polyphonic sequencing.

## Performance

Uses custom global allocator (rlsf) with assert_no_alloc to guarantee zero allocations in audio callback. Benchmarks available in `benches/`.

## Note

The `exploration/` folder contains experimental ideas and should not be used or edited directly.
