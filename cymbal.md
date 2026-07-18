## Cymbal/Hi-Hat Synthesis - Concise Guide

### Core Principle
Cymbals need **inharmonic spectra** - not white noise, not pitched tones. Dense, non-harmonic partials create the metallic character.

### TR-808 Method (Classic)
- **6 square-wave oscillators** at non-harmonic frequencies
- Typical: ~205Hz, ~304Hz, ~370Hz, ~523Hz, ~540Hz, ~800Hz
- Avoid even multiples - the "wrongness" = metallic
- Mix all, then steep **highpass filter** (~6kHz+)

### Signal Path

```
[Osc Bank] -> [Mix] -> [HPF] -> [VCA] -> [Out]
```

For realism, split into frequency bands with separate envelopes (highs decay faster).

### Key Parameters

| Parameter | Function |
|-----------|----------|
| Decay | Envelope time - short=tight, long=washy |
| Tone | Balance between low/high partials |
| HPF Cutoff | Lower=fuller, higher=thinner |

### Sound Variations

| Sound | Decay | Notes |
|-------|-------|-------|
| Closed HH | ~50ms | Short, clicky |
| Open HH | 90-600ms | Sustained ring |
| Crash | 350-1200ms | Explosive, full wash |
| Ride | Medium | Ping + wash, more pitched |

Same oscillator bank for all - differentiation via filtering and envelope.

### Quick Recipe

1. 4-6 square waves at inharmonic intervals
2. Mix -> steep HPF @ ~6kHz
3. VCA with fast attack, variable decay
4. Closed HH: 50ms decay / Open HH: 200ms+ decay

### Tips
- Ring mod between oscillator pairs adds density
- White noise mixed subtly adds "air" for crashes
- Self-oscillating filter adds metallic "ping"
