use brainwash_gui::audio::AudioRuntime;
use brainwash_gui::model::{GuiState, Mode};
use brainwash_gui::view::main_view;
use haven::winit::WinitApp;
use haven::*;

fn main() {
    let mut audio = match AudioRuntime::start() {
        Ok(audio) => Some(audio),
        Err(err) => {
            let reason = format!("{err:?}");
            eprintln!("Audio disabled: {reason}");
            None
        }
    };
    let mut state = GuiState::default();
    match &mut audio {
        Some(audio) => match audio.take_handle() {
            Some(handle) => state.set_audio(handle),
            None => state.set_audio_unavailable("handle unavailable"),
        },
        None => state.set_audio_unavailable("device unavailable"),
    }
    WinitApp::new(state)
        .pane(
            PaneBuilder::new("main", main_view)
                .title("Brainwash")
                .on_frame(frame)
                .inner_size(760, 560),
        )
        .run();
}

fn frame(state: &mut GuiState, app: &mut PaneState) {
    if matches!(state.mode(), Mode::ProbeEdit { .. }) {
        app.redraw();
    }
    if !state.take_load_request() {
        return;
    }
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Brainwash", &["bw"])
        .pick_file()
    else {
        return;
    };
    if let Err(error) = state.load_project(&path) {
        eprintln!("Load failed: {error}");
    }
}
