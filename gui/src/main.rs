use brainwash_gui::audio::AudioRuntime;
use brainwash_gui::model::{GuiState, Mode};
use brainwash_gui::view::main_view;
use haven::winit::WinitApp;
use haven::*;
use std::path::Path;

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
    if state.playing() || matches!(state.mode(), Mode::ProbeEdit { .. }) {
        app.redraw();
    }
    if state.take_save_request() {
        let dialog = rfd::FileDialog::new()
            .add_filter("Brainwash", &["bw"])
            .set_title("Save Brainwash Patch");
        let dialog = if let Some(saved_path) = state.saved_path() {
            let path = Path::new(saved_path);
            let dialog = path
                .parent()
                .map(|parent| dialog.clone().set_directory(parent))
                .unwrap_or(dialog);
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| dialog.clone().set_file_name(name))
                .unwrap_or(dialog)
        } else {
            dialog.set_file_name("patch.bw")
        };
        if let Some(path) = dialog.save_file() {
            let path = if path.extension().is_none() {
                path.with_extension("bw")
            } else {
                path
            };
            let saved = state.save_project(&path);
            let load_after_save = state.take_load_after_save();
            if saved && load_after_save {
                state.apply(brainwash_gui::model::GuiAction::Load);
            }
        } else {
            state.take_load_after_save();
        }
        app.redraw();
        return;
    }
    if let Some(module) = state.take_relink_sample_request() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("WAV audio", &["wav", "wave"])
            .set_title("Relink Sample")
            .pick_file()
        {
            state.relink_sample(module, &path);
        }
        app.redraw();
        return;
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
    let _ = state.load_project(&path);
    app.redraw();
}
