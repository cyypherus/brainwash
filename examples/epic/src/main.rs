use brainwash::*;

fn main() {
    let bpm = 72.;
    let base_bars = 4. * 16.;
    let mut clock = Clock::default();
    clock.bpm(bpm).bars(base_bars);
    
    let scale = cmin();
    
    let mut reverb = Reverb::default();
    let mut lpf = LowpassFilter::default();
    let mut hpf = HighpassFilter::default();
    let mut delay1 = Delay::default();
    let mut delay2 = Delay::default();

    let composition = "
    # =========================================
    # SECTION A: INTRO - Sparse, atmospheric
    # i -> VI -> III -> VII (Cm -> Ab -> Eb -> Bb)
    # 4 bars
    # =========================================
    {
        {0&4&7}
        &
        ((_/_/_/7)/(9/11/12/_)/(_/14/12/11)/(9/7/4/_))
    }
    {
        {5&9&12}
        &
        ((12/_/9/_)/(5/7/9/12)/(_/14/_/12)/(9/_/7/_))
    }
    {
        {2&7&11}
        &
        ((11/14/16/14)/(11/9/7/9)/(11/12/14/16)/(14/12/11/9))
    }
    {
        {-1&4&7}
        &
        ((7/9/11/12)/(14/_/12/_)/(11/9/7/4)/(2/4/7/_))
    }

    # =========================================
    # SECTION B: BUILD - More motion, arpeggios
    # i -> iv -> VI -> V (Cm -> Fm -> Ab -> G)
    # 4 bars with 16th note runs
    # =========================================
    {
        {0&4&7}/{0&4&7}/{0&4&7}/{0&4&7}
        &
        ((0/4/7/12)/(7/4/0/4)/(7/11/12/16)/(12/11/7/4))
    }
    {
        {3&5&8}/{3&5&8}/{3&5&8}/{3&5&8}
        &
        ((3/5/8/12)/(15/12/8/5)/(3/8/12/15)/(12/8/5/3))
    }
    {
        {5&9&12}/{5&9&12}/{5&9&12}/{5&9&12}
        &
        ((5/9/12/16)/(19/16/12/9)/(5/12/16/19)/(16/12/9/5))
    }
    {
        {-2&2&7}/{-2&2&7}/{-2&2&7}/{-2&2&7}
        &
        ((7/11/14/19)/(14/11/7/2)/(7/14/19/23)/(19/14/11/7))
    }

    # =========================================
    # SECTION C: CLIMAX - Full chords, rhythmic
    # i -> VII -> VI -> VII -> i -> iv -> V -> V
    # 8 bars, dense harmonies
    # =========================================
    {
        {0&4&7&12}/{-1&4&7&11}/{5&9&12&16}/{-1&4&7&11}
        &
        ((12/14/16/19)/(16/14/12/11)/(12/16/19/21)/(19/16/14/12))
    }
    {
        {0&4&7&12}/{3&5&8&12}/{-2&2&7&11}/{-2&2&7&11}
        &
        ((12/11/9/7)/(9/12/14/16)/(14/11/7/4)/(7/11/14/19))
    }
    {
        {0&4&7&12}/{0&4&7&12}/{-1&4&7&11}/{-1&4&7&11}
        &
        ((19/21/23/24)/(23/21/19/16)/(14/16/19/21)/(19/16/14/11))
    }
    {
        {5&9&12&16}/{5&9&12&16}/{-2&2&7&11}/{-2&2&7&11}
        &
        ((16/19/21/23)/(21/19/16/14)/(11/14/16/19)/(16/14/11/7))
    }
    {
        {0&4&7&12}/{0&7&12&16}/{5&9&12&16}/{5&12&16&19}
        &
        ((12/16/19/23)/(19/16/12/7)/(16/19/23/26)/(23/19/16/12))
    }
    {
        {3&5&8&12}/{3&8&12&15}/{-2&2&7&11}/{-2&7&11&14}
        &
        ((12/15/19/22)/(19/15/12/8)/(11/14/18/21)/(18/14/11/7))
    }
    {
        {-2&2&7&11}/{-2&2&7&14}/{-2&2&7&11}/{-2&2&7&14}
        &
        ((11/14/18/21)/(18/21/23/26)/(21/18/14/11)/(14/18/21/26))
    }
    {
        {-2&2&7&11}/{-2&2&7&14}/{-2&2&7&18}/{-2&2&7&21}
        &
        ((21/23/26/28)/(26/23/21/18)/(14/18/21/26)/(21/18/14/11))
    }

    # =========================================
    # SECTION D: OUTRO - Return to calm
    # VI -> VII -> i (Ab -> Bb -> Cm) with ritardando feel
    # 4 bars, sparse, fading
    # =========================================
    {
        {5&9&12}
        &
        ((_/12/_/9)/(_/5/_/_)/(9/12/16/_)/(_/_/12/_))
    }
    {
        {-1&4&7}
        &
        ((7/_/4/_)/(_/7/11/_)/(7/4/_/_)/(_/_/4/_))
    }
    {
        {0&4&7}
        &
        ((_/7/_/4)/(7/12/_/7)/(4/_/_/_)/(_/_/_/_))
    }
    {
        {0&4&7}
        &
        ((_/_/12/_)/(_/7/_/_)/(4/_/_/_)/(_/_/_/_))
    }
    ";

    let mut track = Track::parse(composition, &scale).expect("Failed to parse composition");

    let mut keyboard: Keyboard<(
        GateRamp, GateRamp, ADSR,
        GateRamp, ADSR,
        Osc, Osc, Osc, Osc
    )> = Keyboard::with_builder(|| {
        (
            GateRamp::default(),
            GateRamp::default(),
            ADSR::default(),
            GateRamp::default(),
            ADSR::default(),
            Osc::default(),
            Osc::default(),
            Osc::default(),
            Osc::default(),
        )
    });

    play_live(move |s| {
        let phase = clock.output(s);
        let section = (phase * 4.0) as usize;
        
        let mut output = 0.;
        keyboard.update(track.play(phase), s);
        
        keyboard.per_key(
            |(rise, fall, adsr, vib_rise, vib_env, osc_main, osc_fifth, osc_sub, osc_vib),
             key| {
                let gate = if key.pressed() { 1.0 } else { 0.0 };
                
                let attack_time = match section {
                    0 => 0.2,
                    1 => 0.05,
                    2..=3 => 0.01,
                    _ => 0.3,
                };
                let release_time = match section {
                    0 => 0.8,
                    1 => 0.4,
                    2..=3 => 0.2,
                    _ => 1.2,
                };
                
                let rise_val = rise.rise().time(attack_time).output(gate, s);
                let fall_val = fall.fall().time(release_time).output(gate, s);
                let env = adsr.pad().output(rise_val, fall_val);

                let vib_amount = match section {
                    2..=3 => 0.15,
                    _ => 0.05,
                };
                let vib_r = vib_rise.rise().time(0.8).output(gate, s);
                let vib_depth = vib_env.pad().output(vib_r, 0.0) * vib_amount;
                let vibrato = osc_vib.sin().freq(4.5).gain(1.0).output(s) * key.freq * vib_depth;

                let freq = key.freq + vibrato;
                
                let main = osc_main.sin().freq(freq).gain(env).output(s);
                
                let fifth_gain = match section {
                    0 => 0.0,
                    1 => 0.2,
                    2..=3 => 0.4,
                    _ => 0.1,
                };
                let fifth = osc_fifth.tri().freq(freq * 1.5).gain(env * fifth_gain).output(s);
                
                let sub_gain = match section {
                    2..=3 => 0.3,
                    _ => 0.15,
                };
                let sub = osc_sub.sin().freq(freq * 0.5).gain(env * sub_gain).output(s);
                
                output += main + fifth + sub;
            },
        );

        let cutoff = match section {
            0 => 0.15,
            1 => 0.25,
            2..=3 => 0.6,
            _ => 0.1,
        };
        output = lpf.freq(cutoff).q(0.7).output(output, s);
        output = hpf.freq(0.01).output(output, s);

        let tap1 = delay1.delay(((60.0 / bpm) * 0.375 * 44100.0) as f32).tap();
        let tap2 = delay2.delay(((60.0 / bpm) * 0.5 * 44100.0) as f32).tap();
        
        let delay_mix = match section {
            0 => 0.4,
            1 => 0.3,
            2..=3 => 0.2,
            _ => 0.5,
        };
        
        let reverbed = reverb.roomsize(0.85).damp(0.3).output(tap1 + tap2);
        output = delay1.output(output + tap1 * delay_mix * 0.6);
        output = delay2.output(output + tap2 * delay_mix * 0.4);
        output += reverbed * 0.25;

        let master = match section {
            2..=3 => 0.08,
            _ => 0.12,
        };
        
        output * master
    })
    .expect("Error with live audio");
}
