# Modular Synth TUI

## Current State

Grid-based modular synthesizer with:
- Modules connect via channels flowing **down** or **right**
- 6-voice polyphony with voice stealing
- Crossfade on patch changes (441 samples)
- Track notation for sequencing

### Module Set
- **Track**: Freq, Gate
- **Generator**: Osc (sin/squ/tri/saw/rsaw/noise)
- **Envelopes**: ADSR, Envelope (arbitrary points with curve/linear)
- **Effects**: LPF, HPF, Delay, Reverb, Distortion, Flanger
- **Math**: Mul, Add, Gain, Probe
- **Routing**: LSplit, TSplit, RJoin, DJoin, TurnRD, TurnDR
- **Output**: Out

### Keybindings
| Key | Action |
|-----|--------|
| `hjkl` | Move cursor |
| `Space` | Open palette |
| `m` | Grab/drop module |
| `o` | Rotate module |
| `u` | Edit params |
| `.` | Delete |
| `v` | Rectangle select |
| `t` | Edit track |
| `p` | Play/pause |
| `q` | Quit |

## Goals

### 1. Persistence

Save/load patches to disk. File format must be:
- Human-readable (text-based)
- Forward-compatible (ignore unknown fields)
- Partial (can define snippets, not just full patches)

#### File Format: `.bw` (brainwash)

```
# Comment lines start with #
# Blank lines ignored

# Global settings (optional, defaults shown)
bpm 120
bars 1
scale chromatic
root C4

# Module definitions
# module <id> <kind> <x> <y> [orientation]
module 1 Freq 0 0
module 2 Gate 1 0
module 3 Osc 0 2 v
module 4 ADSR 1 2 v
module 5 Mul 0 5
module 6 Out 0 7

# Module parameters (optional, only non-defaults)
# param <id> <index> <value>
param 3 0 1          # Osc waveform = square
param 4 1 0.05       # ADSR attack
param 4 2 0.2        # ADSR decay
param 4 3 0.6        # ADSR sustain
param 4 4 0.4        # ADSR release

# Port connections (which ports accept input)
# port <id> <bitmask>
port 3 0xFF          # all ports open

# Envelope points (for Envelope modules)
# env <id> <time> <value> [curve]
env 7 0.0 0.0
env 7 0.3 1.0 curve
env 7 1.0 0.0

# Track notation (optional)
track
C4 D4 E4 F4
G4 A4 B4 C5
end
```

#### Snippet Format

Same format but without requiring Out module. Can define reusable module groups:

```
# filter_chain.bw - LPF into Reverb snippet
module 1 LPF 0 0 v
module 2 Reverb 0 2 v
param 1 1 0.3
param 2 1 0.7
```

### 2. Global Controls

Add global parameters accessible from status bar or dedicated panel:

| Control | Range | Default | Description |
|---------|-------|---------|-------------|
| BPM | 20-300 | 120 | Tempo |
| Bars | 0.25-16 | 1 | Loop length |
| Scale | enum | chromatic | Note quantization |
| Root | note | C4 | Scale root note |
| Master | 0-2 | 1.0 | Master volume |

#### Scale Options
- chromatic
- major
- minor
- pentatonic
- blues
- dorian
- mixolydian

### 3. Implementation Tasks

- [ ] Parse `.bw` file format
- [ ] Write `.bw` file format
- [ ] `s` key to save (prompt filename if new)
- [ ] `S` key to save as
- [ ] `L` key to load
- [ ] Global controls UI (maybe `g` key)
- [ ] Scale quantization in track playback
- [ ] Snippet paste from file
