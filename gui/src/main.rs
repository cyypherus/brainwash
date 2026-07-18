use brainwash_gui::audio::AudioRuntime;
use brainwash_gui::model::{GuiState, Mode};
use brainwash_gui::view::main_view;
use haven::winit::WinitApp;
use haven::*;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    if let Some(path) = user_modules_dir() {
        state.set_user_compositions(load_modules(&path));
    }
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
    if state.take_open_modules_request() {
        if let Some(path) = user_modules_dir() {
            match std::fs::create_dir_all(&path).and_then(|()| reveal(&path)) {
                Ok(()) => state.set_user_compositions(load_modules(&path)),
                Err(error) => eprintln!("Could not open modules folder: {error}"),
            }
        }
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

fn load_modules(path: &Path) -> Vec<brainwash::patch::Composition> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("bwm"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| match brainwash::persist::load_composition(&path) {
            Ok(composition) => Some(composition),
            Err(_) => {
                eprintln!("Could not load module {}", path.display());
                None
            }
        })
        .collect()
}

fn user_modules_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library/Application Support/Brainwash/Modules"));
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|data| data.join("Brainwash/Modules"));
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".local/share"))
            })
            .map(|data| data.join("brainwash/modules"))
    }
}

fn reveal(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = Command::new("xdg-open");
    command.arg(path).spawn().map(|_| ())
}
