## Kick Drum Synthesis - Concise Guide

### Core Concept
Two simultaneous events:
1. **Pitch sweep** — high→low frequency drop (the "punch")
2. **Amplitude decay** — volume fade (the "boom")

### Basic Modular Patch

| Module | Role |
|--------|------|
| **Sine VCO** | ~40-60Hz fundamental |
| **Pitch Envelope** | Fast exponential decay → VCO pitch |
| **Amp Envelope** | Longer exponential decay → VCA |
| **VCA** | Shapes amplitude |

### Key Parameters

| Parameter | Effect | Range |
|-----------|--------|-------|
| Base pitch | Weight | 40-80Hz |
| Pitch env depth | "zappy" vs "thud" | 1-3 octaves |
| Pitch env decay | Punch sharpness | 5-50ms |
| Amp decay | Boom/sustain | 100-500ms |

### Historical Approaches

**TR-808**: Self-oscillating filter excited by trigger pulse. Simple but less control.

**TR-909**: Adds transient click (short noise burst through LPF, mixed with oscillator via second fast VCA).

### Tips
- Use **exponential** decay curves (linear sounds weak)
- Pitch envelope should be **faster** than amp envelope
- Add filtered noise burst for click/attack
- Post-VCA distortion adds weight
