use brainwash2::*;

fn main() {
    let bpm = 80.0;

    let track_str = "
    (
        {
            (9/11/13)
            %
            {0%2%4}/{3%5%7}/{4%6%8}/{0%2%4}
        }
    )
    ";
    // let track_str = "
    //         {0%2%4}
    // ";

    let track = brainwash2::track::parse(track_str).expect("Failed to parse track");
    let parsed_track = ParsedTrack::from_track(&track, &cmaj());

    let mut instrument = Instrument::new(move || {
        let mut track = parsed_track.clone();
        let mut clock = rsaw();
        let mut osc1 = saw();
        let mut osc2 = tri();
        move |key, _sample_time: usize| {
            let clock_pos = clock.hz(1.).unipolar().play(0.5);

            let press_state = track.query(key, clock_pos);

            // dbg!(key.frequency);
            // gain(
            //     adsr(lead(), press_state),
            //     mix(vec![
            //         osc2.shift(-12.).play(key.frequency),
            //         osc1.shift(-12.).play(key.frequency),
            //     ]),
            // )
            if let PressState::Pressed {
                pressed_at,
                time_in_state,
            } = press_state
            {
                osc1.play(key.frequency)
            } else {
                0.
            }
        }
    });

    play_live(move |sample_time| instrument.output(sample_time) * 0.3)
        .expect("Failed to play live");
}
