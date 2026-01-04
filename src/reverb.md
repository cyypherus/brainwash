# FDN Reverb Implementation

8-channel Feedback Delay Network with modulation, based on Valhalla/Dattorro techniques.

## Architecture

```
Input -> Input Diffusion (4x allpass) -> Tank Split
                                             |
         +-----------------------------------+
         |                                   |
         v                                   v
    [Channel 0-3]                      [Channel 4-7]
         |                                   |
         +---> Householder Matrix <----------+
         |           |                       |
         v           v                       v
    Modulated    Damping                Modulated
     Delays      (LPF)                   Delays
         |           |                       |
         +------> Feedback -----------------+
                     |
                     v
              Stereo Output (decorrelated taps)
```

## Components

### Input Diffusion
4 series allpass filters smear transients before the tank.
Delay times: 142, 107, 379, 277 samples (scaled for sample rate)
Coefficients: 0.75, 0.75, 0.625, 0.625

### Tank (8-channel FDN)
- Householder matrix mixing (energy-preserving, minimal coloration)
- Per-channel modulated delays with cubic interpolation
- Per-channel one-pole lowpass damping
- Cross-coupled feedback for rich, dense tails

### Modulation
LFO-modulated delay times break up metallic resonances.
Each channel has slightly different rate (0.5-0.85 Hz) and depth.
Phase offsets prevent correlation.

### Output
Decorrelated stereo via polarity inversions:
- Left: ch0 + ch2 - ch4 + ch6
- Right: ch1 - ch3 + ch5 + ch7

## Parameters

| Param | Range | Description |
|-------|-------|-------------|
| Room  | 0-1   | Decay time (0.3-0.99 feedback) |
| Damp  | 0-1   | HF absorption (darker sound) |
| Mod   | 0-1   | Modulation depth (0=static, 1=lush) |
| Diff  | 0-1   | Input diffusion (0=echoes, 1=smooth) |

## References

- Dattorro, "Effect Design Part 1" (1997) - figure-8 topology, delay times
- Valhalla DSP blog - modulation, diffusion techniques
- Signalsmith Audio - Householder/Hadamard FDN mixing
