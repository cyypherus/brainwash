# Modular Synth TUI Design

## Vision

A grid-based modular synthesizer interface where modules connect via highlighted channels flowing **down** or **right**. The grid acts as a breadboard - any float output connects to any f32 input. Signal flows from sources toward a single **Output** module in the bottom-right corner.

## Current State

Basic TUI working with:
- Grid navigation (hjkl)
- Module placement from palette (Space)
- Module rotation (o), editing (u), deletion (.)
- Audio playback (p)
- Auto-commit on changes
- Connection tracing and topological sort for audio graph

## Big Goals

### 1. Global Track Input
Replace per-module Track with a **global multiline text editor** for track notation. The track defines the sequence of notes played. This mirrors how tracks work in code - a single track drives the entire patch.

### 2. Voice System with Freq + Gate Inputs
The grid represents a **single voice**. Two global inputs feed into the patch:
- **Freq** (f32): Current note frequency from the track/sequencer
- **Gate** (f32): 0.0 = released, 1.0 = pressed

These appear as special "input" modules that can be placed on the grid and routed to Osc, ADSR, etc.

### 3. ADSR Refactor
Current ADSR takes a `KeyState` enum. Refactor to:
- Take a simple `f32` gate value (0.0 or 1.0)
- Track timing internally (when gate transitions 0→1, start attack; when 1→0, start release)
- Output envelope value 0.0-1.0

This matches how hardware ADSRs work and simplifies the module interface.

### 4. Code Reference
See `exploration/src/live.rs` for how freq/gate work in code:
```rust
let freq = key.map_or(0., |k| k.freq());
let pressed = key.map_or(0., |k| if k.pressed() { 1. } else { 0. });
// freq feeds into Osc
// pressed feeds into ADSR
```

## Module Categories

### Global Inputs (special modules)
| Module | Outputs | Description |
|--------|---------|-------------|
| `Freq` | frequency | Current note frequency from global track |
| `Gate` | 0/1 | Note on/off state |

### Sources
| Module | Outputs | Description |
|--------|---------|-------------|
| `LFO` | signal | Low frequency oscillator |

### Generators
| Module | Inputs | Outputs | Description |
|--------|--------|---------|-------------|
| `Osc` | freq | signal | Waveform oscillator (sin/squ/tri/saw) |

### Envelopes
| Module | Inputs | Outputs | Description |
|--------|--------|---------|-------------|
| `ADSR` | gate | envelope | Attack/Decay/Sustain/Release (refactored) |

### Effects
| Module | Inputs | Outputs | Description |
|--------|--------|---------|-------------|
| `LPF` | signal, cutoff | signal | Low-pass filter |
| `HPF` | signal, cutoff | signal | High-pass filter |
| `Delay` | signal, time | signal | Delay line |
| `Reverb` | signal, mix | signal | Freeverb |
| `Dist` | signal, drive | signal | Soft-clip distortion |
| `Flanger` | signal, depth | signal | Flanging effect |

### Math
| Module | Inputs | Outputs | Description |
|--------|--------|---------|-------------|
| `Mul` | a, b | product | Multiply two signals |
| `Add` | a, b | sum | Sum two signals |

### Routing
| Module | Behavior |
|--------|----------|
| `TurnRD` ┐ | Input left, output down |
| `TurnDR` └ | Input top, output right |
| `LSplit` ◁ | Input left, outputs down + right |
| `TSplit` △ | Input top, outputs down + right |
| `RJoin` ▶ | Inputs left + top, output right |
| `DJoin` ▼ | Inputs left + top, output down |

### Output
| Module | Inputs | Description |
|--------|--------|-------------|
| `Out` | signal | Final audio output |

## Keyboard Shortcuts

### Navigation
| Key | Action |
|-----|--------|
| `hjkl` | Move cursor |
| `HJKL` | Jump 4 cells |
| `[` | Jump to origin (0,0) |
| `]` | Jump to output module |

### Module Operations
| Key | Action |
|-----|--------|
| `Space` | Open module palette |
| `m/Enter` | Grab module to move |
| `o` | Rotate module |
| `u` | Edit module params |
| `.` | Delete module |

### Edit Mode
| Key | Action |
|-----|--------|
| `jk` | Select param |
| `hl` | Adjust value |
| `HL` | Adjust value (10x) |
| `;` | Toggle port open/closed |
| `Enter/Esc` | Exit edit mode |

### Playback
| Key | Action |
|-----|--------|
| `p` | Play/Pause |

### Palette Shortcuts
| Key | Category |
|-----|----------|
| `7` | Sources |
| `8` | Generators |
| `9` | Envelopes |
| `0` | Effects |
| `-` | Math |
| `=` | Routing |

## Implementation Tasks

### Immediate
- [ ] Remove: Mix, Ramp, Const, Track, Keyboard, Noise, Clock modules
- [ ] Add: Freq and Gate global input modules
- [ ] Refactor ADSR to take f32 gate instead of KeyState

### Track System
- [ ] Global track notation editor (multiline text)
- [ ] Track parser → (freq, gate) output per sample
- [ ] Wire Freq/Gate modules to track output

### Future
- [ ] Patch save/load
- [ ] Multiple voices / polyphony
- [ ] More oscillator types
- [ ] Modulation routing visualization
