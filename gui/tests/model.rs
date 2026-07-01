use brainwash::compile::PatchControls;
use brainwash::time::{Hertz, SampleRate};
use brainwash_gui::model::{
    GridPos, GuiAction, GuiState, Mode, ModuleCategory, ModuleKind, Orientation, ParameterValue,
    TimeUnit, all_modules,
};
use brainwash_gui::view::main_view;
use haven::{Key, NamedKey, PaneBuilder, Point};
use std::fs;

#[test]
fn module_inventory_matches_tui_surface_count() {
    assert_eq!(all_modules().len(), 38);
    assert_eq!(
        all_modules()
            .iter()
            .filter(|kind| kind.category() == ModuleCategory::Source)
            .count(),
        7
    );
    assert_eq!(
        all_modules()
            .iter()
            .filter(|kind| kind.category() == ModuleCategory::Routing)
            .count(),
        6
    );
}

#[test]
fn cursor_movement_stays_inside_grid() {
    let mut state = GuiState::new(4, 3);

    state.apply(GuiAction::Left);
    state.apply(GuiAction::Up);
    assert_eq!(state.cursor(), GridPos::new(0, 0));

    state.apply(GuiAction::RightFast);
    state.apply(GuiAction::DownFast);
    assert_eq!(state.cursor(), GridPos::new(3, 2));
}

#[test]
fn fast_movement_matches_tui_step() {
    let mut state = GuiState::new(10, 10);

    state.apply(GuiAction::RightFast);
    state.apply(GuiAction::DownFast);

    assert_eq!(state.cursor(), GridPos::new(4, 4));
}

#[test]
fn palette_places_selected_module_at_cursor() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::PaletteDown);
    state.apply(GuiAction::Confirm);

    let module = state.module_at(GridPos::new(1, 1)).unwrap();
    assert_eq!(module.kind(), ModuleKind::Gate);
    assert_eq!(module.orientation(), Orientation::Right);
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn palette_category_changes_reset_selection() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::PaletteRight);

    assert_eq!(state.palette_category(), ModuleCategory::Shape);
    assert_eq!(state.selected_palette_module(), ModuleKind::Rise);
}

#[test]
fn direct_palette_category_opens_and_resets_selection() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::Palette(ModuleCategory::Effect));
    state.apply(GuiAction::PaletteDown);
    assert_eq!(state.mode(), Mode::Palette);
    assert_eq!(state.palette_category(), ModuleCategory::Effect);
    assert_ne!(state.selected_palette_module(), ModuleKind::Comb);

    state.apply(GuiAction::Palette(ModuleCategory::Filter));
    assert_eq!(state.palette_category(), ModuleCategory::Filter);
    assert_eq!(state.selected_palette_module(), ModuleKind::Lowpass);
}

#[test]
fn palette_search_filters_and_places_module() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::Search);
    state.apply(GuiAction::InputChar('l'));
    state.apply(GuiAction::InputChar('p'));
    state.apply(GuiAction::InputChar('f'));

    assert!(state.palette_searching());
    assert_eq!(state.palette_filter(), "lpf");
    assert_eq!(
        state.selected_filtered_palette_module(),
        Some(ModuleKind::Lowpass)
    );

    state.apply(GuiAction::Confirm);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Lowpass
    );
    assert_eq!(state.mode(), Mode::Normal);
    assert!(!state.palette_searching());
}

#[test]
fn palette_search_navigation_backspace_and_cancel_match_tui() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("n"));
    pane.key_pressed(&mut state, Key::character("/"));
    pane.key_pressed(&mut state, Key::character("p"));
    assert!(state.filtered_palette_modules().len() > 1);

    let first = state.selected_filtered_palette_module();
    pane.key_pressed(&mut state, NamedKey::ArrowDown);
    assert_ne!(state.selected_filtered_palette_module(), first);
    pane.key_pressed(&mut state, NamedKey::ArrowUp);
    assert_eq!(state.selected_filtered_palette_module(), first);

    pane.key_pressed(&mut state, NamedKey::Backspace);
    assert_eq!(state.palette_filter(), "");
    assert_eq!(state.selected_filtered_palette_module(), None);

    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, Key::character("p"));
    pane.key_pressed(&mut state, Key::character("f"));
    assert_eq!(
        state.selected_filtered_palette_module(),
        Some(ModuleKind::Lowpass)
    );

    pane.key_pressed(&mut state, NamedKey::Escape);
    assert_eq!(state.mode(), Mode::Palette);
    assert!(!state.palette_searching());
    assert_eq!(state.palette_filter(), "");
    assert_eq!(state.module_at(GridPos::new(0, 0)), None);
}

#[test]
fn select_action_cancels_selection_mode() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    assert_eq!(
        state.selection(),
        Some((GridPos::new(0, 0), GridPos::new(1, 0)))
    );

    state.apply(GuiAction::Select);
    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.selection(), None);
}

#[test]
fn move_confirm_cancel_delete_and_rotate_match_tui_basics() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::Confirm);
    let id = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Rotate);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().orientation(),
        Orientation::Down
    );

    state.apply(GuiAction::Move);
    assert_eq!(
        state.mode(),
        Mode::Move {
            module: id,
            origin: GridPos::new(0, 0)
        }
    );
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Cancel);
    assert!(state.module_at(GridPos::new(0, 0)).is_some());

    state.apply(GuiAction::Move);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Confirm);
    assert!(state.module_at(GridPos::new(1, 1)).is_some());

    state.apply(GuiAction::Delete);
    assert!(state.modules().is_empty());
}

#[test]
fn copy_places_module_and_rejects_occupied_target() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    let source = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Copy);
    assert_eq!(state.mode(), Mode::Copy { source });
    state.apply(GuiAction::Confirm);
    assert_eq!(state.modules().len(), 1);
    assert_eq!(state.mode(), Mode::Copy { source });

    state.apply(GuiAction::Right);
    state.apply(GuiAction::Confirm);

    assert_eq!(state.modules().len(), 2);
    assert!(state.module_at(GridPos::new(0, 0)).is_some());
    assert!(state.module_at(GridPos::new(1, 0)).is_some());
    assert_ne!(
        state.module_at(GridPos::new(0, 0)).unwrap().id(),
        state.module_at(GridPos::new(1, 0)).unwrap().id()
    );
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn copy_cancel_leaves_original_only() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);

    state.apply(GuiAction::Copy);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Cancel);

    assert_eq!(state.modules().len(), 1);
    assert!(state.module_at(GridPos::new(0, 0)).is_some());
    assert!(state.module_at(GridPos::new(1, 0)).is_none());
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn edit_mode_adjusts_parameters_ports_units_and_undoes() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    assert_eq!(
        state.mode(),
        Mode::Edit {
            module,
            parameter: 0
        }
    );

    state.apply(GuiAction::Down);
    assert_eq!(
        state.mode(),
        Mode::Edit {
            module,
            parameter: 1
        }
    );

    state.apply(GuiAction::CycleUnit);
    let parameter = &state.module_at(GridPos::new(0, 0)).unwrap().parameters()[1];
    assert_eq!(
        parameter.value(),
        &ParameterValue::Time {
            value: 100,
            unit: TimeUnit::Samples
        }
    );

    state.apply(GuiAction::ValueUp);
    let parameter = &state.module_at(GridPos::new(0, 0)).unwrap().parameters()[1];
    assert_eq!(
        parameter.value(),
        &ParameterValue::Time {
            value: 102,
            unit: TimeUnit::Samples
        }
    );
    assert!(!parameter.connected());

    state.apply(GuiAction::TogglePort);
    assert!(state.module_at(GridPos::new(0, 0)).unwrap().parameters()[1].connected());

    state.apply(GuiAction::Cancel);
    state.apply(GuiAction::Undo);
    let parameter = &state.module_at(GridPos::new(0, 0)).unwrap().parameters()[1];
    assert_eq!(
        parameter.value(),
        &ParameterValue::Time {
            value: 102,
            unit: TimeUnit::Samples
        }
    );
    assert!(!parameter.connected());
}

#[test]
fn typed_value_input_commits_numeric_parameter() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::TypeValue);
    assert_eq!(
        state.mode(),
        Mode::ValueInput {
            module,
            parameter: 1
        }
    );

    state.apply(GuiAction::Backspace);
    state.apply(GuiAction::InputChar('2'));
    state.apply(GuiAction::InputChar('.'));
    state.apply(GuiAction::InputChar('5'));
    state.apply(GuiAction::Confirm);

    assert_eq!(
        state.mode(),
        Mode::Edit {
            module,
            parameter: 1
        }
    );
    let parameter = &state.module_at(GridPos::new(0, 0)).unwrap().parameters()[1];
    assert_eq!(
        parameter.value(),
        &ParameterValue::Time {
            value: 250,
            unit: TimeUnit::Seconds
        }
    );
    assert!(!parameter.connected());
}

#[test]
fn typed_value_input_filters_characters_like_tui() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::TypeValue);
    assert_eq!(
        state.mode(),
        Mode::ValueInput {
            module,
            parameter: 1
        }
    );
    assert_eq!(state.prompt_text(), "1");
    state.apply(GuiAction::InputChar('/'));
    state.apply(GuiAction::InputChar('a'));
    assert_eq!(state.prompt_text(), "1/");
    state.apply(GuiAction::Cancel);

    state.apply(GuiAction::Down);
    state.apply(GuiAction::TypeValue);
    assert_eq!(
        state.mode(),
        Mode::ValueInput {
            module,
            parameter: 2
        }
    );
    assert_eq!(state.prompt_text(), "0");
    state.apply(GuiAction::InputChar('/'));
    state.apply(GuiAction::InputChar('a'));
    state.apply(GuiAction::InputChar('5'));
    assert_eq!(state.prompt_text(), "05");
}

#[test]
fn save_export_and_quit_prompts_track_requested_state() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::Quit);
    assert!(state.should_quit());

    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    assert!(state.dirty());

    state.apply(GuiAction::Quit);
    assert_eq!(state.mode(), Mode::QuitConfirm);
    state.apply(GuiAction::Cancel);
    assert_eq!(state.mode(), Mode::Normal);

    state.apply(GuiAction::Save);
    assert_eq!(state.mode(), Mode::SavePrompt);
    assert_eq!(state.prompt_text(), "patch.bw");
    state.apply(GuiAction::Confirm);
    assert_eq!(state.saved_path(), Some("patch.bw"));
    assert!(!state.dirty());

    state.apply(GuiAction::Export);
    assert_eq!(state.mode(), Mode::ExportPrompt);
    assert_eq!(state.prompt_text(), "patch.wav");
    state.apply(GuiAction::Up);
    assert_eq!(state.export_loops(), 2);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.exported_path(), Some("patch.wav"));
}

#[test]
fn save_and_export_prompt_confirm_existing_paths_before_overwrite() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::Save);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.saved_path(), Some("patch.bw"));

    state.apply(GuiAction::Export);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.exported_path(), Some("patch.wav"));

    state.apply(GuiAction::SaveAs);
    assert_eq!(state.mode(), Mode::SavePrompt);
    state.apply(GuiAction::TextStart);
    for _ in 0..8 {
        state.apply(GuiAction::DeleteChar);
    }
    for character in "patch.wav".chars() {
        state.apply(GuiAction::InputChar(character));
    }
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::SaveConfirm);
    state.apply(GuiAction::Cancel);
    assert_eq!(state.mode(), Mode::SavePrompt);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::SaveConfirm);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.saved_path(), Some("patch.wav"));

    state.apply(GuiAction::SaveAs);
    state.apply(GuiAction::TextStart);
    for _ in 0..9 {
        state.apply(GuiAction::DeleteChar);
    }
    for character in "patch.bw".chars() {
        state.apply(GuiAction::InputChar(character));
    }
    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.saved_path(), Some("patch.bw"));

    state.apply(GuiAction::Export);
    state.apply(GuiAction::TextStart);
    for _ in 0..12 {
        state.apply(GuiAction::DeleteChar);
    }
    for character in "patch.bw".chars() {
        state.apply(GuiAction::InputChar(character));
    }
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::ExportConfirm);
    state.apply(GuiAction::Cancel);
    assert_eq!(state.mode(), Mode::ExportPrompt);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::ExportConfirm);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.exported_path(), Some("patch.bw"));
}

#[test]
fn track_settings_match_tui_bounds_and_wrapping() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::TrackSettings);
    assert_eq!(state.mode(), Mode::TrackSettings { parameter: 0 });
    state.apply(GuiAction::ValueUp);
    assert_eq!(state.bpm(), 125);
    state.apply(GuiAction::ValueDownFast);
    assert_eq!(state.bpm(), 105);

    state.apply(GuiAction::Down);
    assert_eq!(state.mode(), Mode::TrackSettings { parameter: 1 });
    state.apply(GuiAction::ValueUp);
    assert_eq!(state.scale_index(), 3);
    assert_eq!(state.scale_name(), "C# maj");

    state.apply(GuiAction::Down);
    assert_eq!(state.mode(), Mode::TrackSettings { parameter: 2 });
    state.apply(GuiAction::ValueDown);
    assert_eq!(state.probe_voice(), 5);

    state.apply(GuiAction::Cancel);
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn track_edit_opens_track_prompt() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::TrackEdit);

    assert!(!state.track_edit_requested());
    assert_eq!(state.mode(), Mode::TrackPrompt);
}

#[test]
fn subpatch_navigation_uses_isolated_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    let subpatch = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::EditSubpatch);
    assert_eq!(state.subpatch_depth(), 1);
    assert_eq!(state.cursor(), GridPos::new(0, 0));
    assert!(state.modules().is_empty());

    state.apply(GuiAction::OpenPalette);
    for _ in 0..6 {
        state.apply(GuiAction::PaletteLeft);
    }
    state.apply(GuiAction::PaletteDown);
    state.apply(GuiAction::Confirm);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );

    state.apply(GuiAction::ExitSubpatch);
    assert_eq!(state.subpatch_depth(), 0);
    assert_eq!(state.cursor(), GridPos::new(0, 0));
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Subpatch
    );

    state.apply(GuiAction::EditSubpatch);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );
    assert_ne!(state.module_at(GridPos::new(0, 0)).unwrap().id(), subpatch);
}

#[test]
fn moving_module_can_enter_subpatch_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::OpenPalette);
    for _ in 0..6 {
        state.apply(GuiAction::PaletteLeft);
    }
    state.apply(GuiAction::PaletteDown);
    state.apply(GuiAction::Confirm);
    let module = state.module_at(GridPos::new(1, 0)).unwrap().id();

    state.apply(GuiAction::Move);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::EditSubpatch);
    assert_eq!(state.subpatch_depth(), 1);
    assert!(state.modules().is_empty());

    state.apply(GuiAction::Confirm);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );
    state.apply(GuiAction::ExitSubpatch);
    assert!(state.module_at(GridPos::new(1, 0)).is_none());
    state.apply(GuiAction::EditSubpatch);
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().id(), module);
}

#[test]
fn cancelling_subpatch_move_restores_module_to_origin_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::OpenPalette);
    for _ in 0..6 {
        state.apply(GuiAction::PaletteLeft);
    }
    state.apply(GuiAction::PaletteDown);
    state.apply(GuiAction::Confirm);
    let module = state.module_at(GridPos::new(1, 0)).unwrap().id();

    state.apply(GuiAction::Move);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::EditSubpatch);
    state.apply(GuiAction::Cancel);

    assert_eq!(state.subpatch_depth(), 0);
    assert_eq!(state.cursor(), GridPos::new(1, 0));
    assert_eq!(state.module_at(GridPos::new(1, 0)).unwrap().id(), module);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::EditSubpatch);
    assert!(state.modules().is_empty());
}

#[test]
fn moving_selection_can_enter_subpatch_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 1);
    let first = state.module_at(GridPos::new(1, 0)).unwrap().id();
    let second = state.module_at(GridPos::new(2, 0)).unwrap().id();

    state.apply(GuiAction::Left);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::LeftFast);
    state.apply(GuiAction::EditSubpatch);
    assert_eq!(state.subpatch_depth(), 1);
    assert!(state.modules().is_empty());

    state.apply(GuiAction::Right);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().id(), first);
    assert_eq!(state.module_at(GridPos::new(1, 0)).unwrap().id(), second);
    state.apply(GuiAction::ExitSubpatch);
    assert!(state.module_at(GridPos::new(1, 0)).is_none());
    assert!(state.module_at(GridPos::new(2, 0)).is_none());
}

#[test]
fn cancelling_subpatch_selection_move_restores_origin_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 1);
    let first = state.module_at(GridPos::new(1, 0)).unwrap().id();
    let second = state.module_at(GridPos::new(2, 0)).unwrap().id();

    state.apply(GuiAction::Left);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::LeftFast);
    state.apply(GuiAction::EditSubpatch);
    state.apply(GuiAction::Cancel);

    assert_eq!(state.subpatch_depth(), 0);
    assert_eq!(state.cursor(), GridPos::new(2, 0));
    assert_eq!(state.module_at(GridPos::new(1, 0)).unwrap().id(), first);
    assert_eq!(state.module_at(GridPos::new(2, 0)).unwrap().id(), second);
    state.apply(GuiAction::LeftFast);
    state.apply(GuiAction::EditSubpatch);
    assert!(state.modules().is_empty());
}

#[test]
fn haven_key_gestures_drive_model() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, Key::character("j"));
    pane.key_pressed(&mut state, Key::character("n"));
    pane.key_pressed(&mut state, NamedKey::Enter);

    assert_eq!(state.cursor(), GridPos::new(1, 1));
    assert!(state.module_at(GridPos::new(1, 1)).is_some());

    pane.key_pressed(&mut state, Key::character("y"));
    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, NamedKey::Enter);

    assert!(state.module_at(GridPos::new(2, 1)).is_some());

    pane.key_pressed(&mut state, NamedKey::Space);
    pane.key_pressed(&mut state, Key::character("v"));
    assert!(state.playing());
    assert!(state.show_meters());
}

#[test]
fn haven_transient_mode_confirm_keys_match_tui() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("n"));
    assert_eq!(state.mode(), Mode::Palette);
    pane.key_pressed(&mut state, Key::character("i"));
    assert_eq!(state.mode(), Mode::Normal);

    pane.key_pressed(&mut state, Key::character("n"));
    pane.key_pressed(&mut state, Key::character("j"));
    pane.key_pressed(&mut state, Key::character("n"));
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );

    pane.key_pressed(&mut state, Key::character("m"));
    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, Key::character("m"));
    assert!(state.module_at(GridPos::new(1, 0)).is_some());

    pane.key_pressed(&mut state, Key::character("y"));
    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, Key::character("y"));
    assert!(state.module_at(GridPos::new(2, 0)).is_some());

    pane.key_pressed(&mut state, Key::character(","));
    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, Key::character(","));
    assert_eq!(state.mode(), Mode::Normal);

    pane.key_pressed(&mut state, Key::character(","));
    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert!(matches!(state.mode(), Mode::SelectMove { .. }));
    pane.key_pressed(&mut state, NamedKey::Escape);

    pane.key_pressed(&mut state, Key::character("Q"));
    pane.key_pressed(&mut state, Key::character("n"));
    assert_eq!(state.mode(), Mode::Normal);
    pane.key_pressed(&mut state, Key::character("Q"));
    pane.key_pressed(&mut state, Key::character("Y"));
    assert!(state.should_quit());
}

#[test]
fn haven_overwrite_confirm_keys_match_tui() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("w"));
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.saved_path(), Some("patch.bw"));

    pane.key_pressed(&mut state, Key::character("e"));
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.exported_path(), Some("patch.wav"));

    pane.key_pressed(&mut state, Key::character("W"));
    for _ in 0..8 {
        pane.key_pressed(&mut state, NamedKey::Backspace);
    }
    for character in "patch.wav".chars() {
        pane.key_pressed(&mut state, Key::character(character.to_string()));
    }
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.mode(), Mode::SaveConfirm);
    pane.key_pressed(&mut state, Key::character("n"));
    assert_eq!(state.mode(), Mode::SavePrompt);
    pane.key_pressed(&mut state, NamedKey::Enter);
    pane.key_pressed(&mut state, Key::character("y"));
    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.saved_path(), Some("patch.wav"));
}

#[test]
fn haven_palette_search_keys_drive_model() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("n"));
    pane.key_pressed(&mut state, Key::character("/"));
    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, Key::character("p"));
    pane.key_pressed(&mut state, Key::character("f"));
    pane.key_pressed(&mut state, NamedKey::Enter);

    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Lowpass
    );
}

#[test]
fn haven_palette_category_keys_match_tui_action() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("@"));
    assert_eq!(state.mode(), Mode::Palette);
    assert_eq!(state.palette_category(), ModuleCategory::Shape);
    assert_eq!(state.selected_palette_module(), ModuleKind::Rise);

    pane.key_pressed(&mut state, Key::character("#"));
    assert_eq!(state.palette_category(), ModuleCategory::Filter);
    assert_eq!(state.selected_palette_module(), ModuleKind::Lowpass);

    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Lowpass
    );
}

#[test]
fn haven_palette_lays_out_categories_horizontally_and_modules_vertically() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::OpenPalette);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);

    let source = pane.location(1_000).unwrap();
    let shape = pane.location(1_001).unwrap();
    assert!(shape.x > source.x);
    assert_point_near_y(shape, source.y);

    let freq = pane.location(2_000).unwrap();
    let gate = pane.location(2_001).unwrap();
    assert_point_near_x(gate, freq.x);
    assert!(gate.y > freq.y);

}

#[test]
fn haven_palette_background_follows_browser_contents() {
    let mut normal = GuiState::new(8, 8);
    normal.apply(GuiAction::OpenPalette);
    let mut normal_pane = PaneBuilder::new("main", main_view).build();
    normal_pane.redraw(&mut normal, 760, 560, 1.0);
    let normal_background = normal_pane.location(200).unwrap();

    let mut search = GuiState::new(8, 8);
    search.apply(GuiAction::OpenPalette);
    search.apply(GuiAction::Search);
    for character in "lpf".chars() {
        search.apply(GuiAction::InputChar(character));
    }
    let mut search_pane = PaneBuilder::new("main", main_view).build();
    search_pane.redraw(&mut search, 760, 560, 1.0);
    let search_background = search_pane.location(200).unwrap();

    assert!(
        search_background.x > normal_background.x + 20.,
        "{search_background:?} {normal_background:?}"
    );
}

#[test]
fn haven_prompt_keys_drive_model() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("w"));
    pane.key_pressed(&mut state, Key::character("e"));
    pane.key_pressed(&mut state, Key::character("w"));
    pane.key_pressed(&mut state, Key::character("n"));
    assert_eq!(state.prompt_text(), "patch.bwewn");
    pane.key_pressed(&mut state, Key::character("C"));
    pane.key_pressed(&mut state, Key::character("G"));
    pane.key_pressed(&mut state, Key::character("@"));
    pane.key_pressed(&mut state, Key::character("!"));
    assert_eq!(state.prompt_text(), "patch.bwewnCG@!");
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.saved_path(), Some("patch.bwewnCG@!"));

    pane.key_pressed(&mut state, Key::character("e"));
    pane.key_pressed(&mut state, NamedKey::ArrowUp);
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.exported_path(), Some("patch.bwewnCG@!.wav"));
    assert_eq!(state.export_loops(), 2);

    pane.key_pressed(&mut state, Key::character("Q"));
    assert!(state.should_quit());
}

#[test]
fn haven_prompt_editing_keys_match_tui_text_input() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("w"));
    assert_eq!(state.mode(), Mode::SavePrompt);
    assert_eq!(state.prompt_text(), "patch.bw");

    pane.key_pressed(&mut state, NamedKey::Home);
    assert_eq!(state.prompt_cursor(), 0);
    pane.key_pressed(&mut state, NamedKey::Delete);
    assert_eq!(state.prompt_text(), "atch.bw");

    pane.key_pressed(&mut state, NamedKey::End);
    assert_eq!(state.prompt_cursor(), "atch.bw".len());
    pane.key_pressed(&mut state, NamedKey::Backspace);
    assert_eq!(state.prompt_text(), "atch.b");
    pane.key_pressed(&mut state, NamedKey::Space);
    assert_eq!(state.prompt_text(), "atch.b ");
    assert!(!state.playing());
    pane.key_pressed(&mut state, NamedKey::Backspace);
    assert_eq!(state.prompt_text(), "atch.b");

    pane.key_pressed(&mut state, NamedKey::ArrowLeft);
    pane.key_pressed(&mut state, Key::character("/"));
    pane.key_pressed(&mut state, Key::character("."));
    assert_eq!(state.prompt_text(), "atch./.b");

    pane.key_pressed(&mut state, NamedKey::Escape);
    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.saved_path(), None);
}

#[test]
fn haven_settings_key_is_contextual() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("s"));
    assert_eq!(state.mode(), Mode::TrackSettings { parameter: 0 });
    pane.key_pressed(&mut state, Key::character("l"));
    assert_eq!(state.bpm(), 125);
    pane.key_pressed(&mut state, NamedKey::Space);
    assert!(state.playing());
    pane.key_pressed(&mut state, Key::character("s"));
    assert_eq!(state.mode(), Mode::Normal);

    pane.key_pressed(&mut state, Key::character("s"));
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.mode(), Mode::Normal);

    pane.key_pressed(&mut state, Key::character("s"));
    pane.key_pressed(&mut state, NamedKey::Escape);

    place_module(&mut state, 0, 4);
    pane.redraw(&mut state, 640, 480, 1.0);
    pane.key_pressed(&mut state, Key::character("i"));
    pane.key_pressed(&mut state, Key::character("s"));
    assert_eq!(state.step_size(), 2);
}

#[test]
fn haven_track_edit_key_is_contextual() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("t"));
    assert_eq!(state.mode(), Mode::TrackPrompt);

    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();
    pane.redraw(&mut state, 640, 480, 1.0);
    pane.key_pressed(&mut state, Key::character("i"));
    pane.key_pressed(&mut state, Key::character("j"));
    pane.key_pressed(&mut state, Key::character("t"));
    assert_eq!(
        state.mode(),
        Mode::ValueInput {
            module,
            parameter: 1
        }
    );
    assert!(!state.track_edit_requested());
}

#[test]
fn haven_track_prompt_accepts_track_notation_keys() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("t"));
    assert_eq!(state.mode(), Mode::TrackPrompt);
    pane.redraw(&mut state, 640, 480, 1.0);
    let field = pane.location(794).expect("track text field present");
    pane.click(&mut state, field);
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, NamedKey::Home);
    for _ in 0..9 {
        pane.key_pressed(&mut state, NamedKey::Delete);
    }
    for character in "{0+/_&7*}".chars() {
        pane.key_pressed(&mut state, Key::character(character.to_string()));
    }
    pane.key_pressed(&mut state, NamedKey::Enter);

    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.track_text(), "{0+/_&7*}");
}

#[test]
fn haven_subpatch_key_enters_and_exits_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("p"));
    assert_eq!(state.subpatch_depth(), 1);

    pane.key_pressed(&mut state, Key::character("p"));
    assert_eq!(state.subpatch_depth(), 0);
}

#[test]
fn haven_move_key_can_place_module_inside_subpatch() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::OpenPalette);
    for _ in 0..6 {
        state.apply(GuiAction::PaletteLeft);
    }
    state.apply(GuiAction::PaletteDown);
    state.apply(GuiAction::Confirm);
    let module = state.module_at(GridPos::new(1, 0)).unwrap().id();
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("m"));
    pane.key_pressed(&mut state, Key::character("h"));
    pane.key_pressed(&mut state, Key::character("p"));
    assert_eq!(state.subpatch_depth(), 1);

    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().id(), module);
}

#[test]
fn haven_selection_move_key_can_place_group_inside_subpatch() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 1);
    let first = state.module_at(GridPos::new(1, 0)).unwrap().id();
    let second = state.module_at(GridPos::new(2, 0)).unwrap().id();
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("h"));
    pane.key_pressed(&mut state, Key::character(","));
    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, Key::character("m"));
    pane.key_pressed(&mut state, Key::character("H"));
    pane.key_pressed(&mut state, Key::character("p"));
    assert_eq!(state.subpatch_depth(), 1);

    pane.key_pressed(&mut state, Key::character("l"));
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().id(), first);
    assert_eq!(state.module_at(GridPos::new(1, 0)).unwrap().id(), second);
}

#[test]
fn haven_edit_keys_drive_parameter_editor() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("i"));
    pane.key_pressed(&mut state, Key::character("j"));
    pane.key_pressed(&mut state, Key::character("u"));

    assert_eq!(
        state.mode(),
        Mode::Edit {
            module,
            parameter: 1,
        }
    );
    assert!(matches!(
        state.module_at(GridPos::new(0, 0)).unwrap().parameters()[1].value(),
        ParameterValue::Time {
            unit: TimeUnit::Samples,
            ..
        }
    ));
    pane.key_pressed(&mut state, NamedKey::Space);
    assert!(state.playing());
    assert_eq!(
        state.mode(),
        Mode::Edit {
            module,
            parameter: 1,
        }
    );
}

#[test]
fn haven_module_info_modal_does_not_duplicate_bottom_shortcuts() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    state.apply(GuiAction::Edit);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);

    assert!(pane.location(501).is_some());
    assert!(pane.location(70_000).is_some());
    assert!(pane.location(3_048).is_none());
    assert!(pane.location(3_052).is_none());
    assert!(pane.location(3_055).is_none());
}

#[test]
fn haven_visual_editors_redraw() {
    let mut pane = PaneBuilder::new("main", main_view).build();

    let mut adsr = GuiState::new(8, 8);
    place_module(&mut adsr, 1, 3);
    adsr.apply(GuiAction::Edit);
    for _ in 0..4 {
        adsr.apply(GuiAction::Down);
    }
    adsr.apply(GuiAction::Confirm);
    pane.redraw(&mut adsr, 920, 720, 1.0);

    let mut envelope = GuiState::new(8, 8);
    place_module(&mut envelope, 1, 4);
    envelope.apply(GuiAction::Edit);
    envelope.apply(GuiAction::Down);
    envelope.apply(GuiAction::Confirm);
    pane.redraw(&mut envelope, 920, 720, 1.0);
    assert!(pane.location(842).is_some());
    assert!(pane.location(841).is_some());

    let mut probe = GuiState::new(8, 8);
    place_module(&mut probe, 4, 0);
    probe.apply(GuiAction::Edit);
    probe.apply(GuiAction::Down);
    probe.apply(GuiAction::Confirm);
    pane.redraw(&mut probe, 920, 720, 1.0);
    assert!(pane.location(901).is_some());
    assert!(pane.location(902).is_some());

    let mut sample = GuiState::new(8, 8);
    place_module(&mut sample, 0, 6);
    sample.apply(GuiAction::Edit);
    sample.apply(GuiAction::Down);
    sample.apply(GuiAction::Down);
    sample.apply(GuiAction::Confirm);
    pane.redraw(&mut sample, 920, 720, 1.0);
}

#[test]
fn haven_envelope_move_keys_match_tui_bindings() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 1, 4);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    pane.key_pressed(&mut state, Key::character("i"));
    pane.key_pressed(&mut state, Key::character("j"));
    pane.key_pressed(&mut state, NamedKey::Enter);
    pane.key_pressed(&mut state, Key::character("m"));
    assert_eq!(
        state.mode(),
        Mode::EnvEdit {
            module,
            point: 0,
            editing: true,
        }
    );

    pane.key_pressed(&mut state, NamedKey::Space);
    assert!(state.playing());
    assert_eq!(
        state.mode(),
        Mode::EnvEdit {
            module,
            point: 0,
            editing: true,
        }
    );

    pane.key_pressed(&mut state, Key::character("l"));
    assert!(state.module_at(GridPos::new(0, 0)).unwrap().env_points()[0].time > 0);
    pane.key_pressed(&mut state, Key::character("s"));
    assert_eq!(state.step_size(), 2);
    pane.key_pressed(&mut state, Key::character("m"));
    assert_eq!(
        state.mode(),
        Mode::EnvEdit {
            module,
            point: 0,
            editing: false,
        }
    );
}

#[test]
fn haven_probe_keys_match_tui_bindings() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 4, 0);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    pane.key_pressed(&mut state, Key::character("i"));
    pane.key_pressed(&mut state, Key::character("j"));
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.mode(), Mode::ProbeEdit { module });

    pane.key_pressed(&mut state, Key::character("k"));
    assert_eq!(state.probe_len(), 8820);
    pane.key_pressed(&mut state, Key::character("j"));
    assert_eq!(state.probe_len(), 4410);
    pane.key_pressed(&mut state, Key::character("r"));
    assert_eq!(state.probe_len(), 4410);
    pane.key_pressed(&mut state, NamedKey::Space);
    assert!(state.playing());
    assert_eq!(state.mode(), Mode::ProbeEdit { module });
    pane.key_pressed(&mut state, Key::character("i"));
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn haven_sample_keys_match_tui_bindings() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 6);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    pane.key_pressed(&mut state, Key::character("i"));
    pane.key_pressed(&mut state, Key::character("j"));
    pane.key_pressed(&mut state, Key::character("j"));
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 1,
            offset: 0,
        }
    );

    pane.key_pressed(&mut state, Key::character("k"));
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 2,
            offset: 0,
        }
    );
    pane.key_pressed(&mut state, Key::character("l"));
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 2,
            offset: 5,
        }
    );
    pane.key_pressed(&mut state, Key::character("j"));
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 1,
            offset: 0,
        }
    );
    pane.key_pressed(&mut state, Key::character("k"));
    pane.key_pressed(&mut state, Key::character("r"));
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 1,
            offset: 0,
        }
    );
    pane.key_pressed(&mut state, NamedKey::Space);
    assert!(state.playing());
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 1,
            offset: 0,
        }
    );
    pane.key_pressed(&mut state, Key::character("i"));
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn adsr_special_editor_adjusts_attack_and_sustain() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 1, 3);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    for _ in 0..4 {
        state.apply(GuiAction::Down);
    }
    state.apply(GuiAction::Confirm);
    assert_eq!(
        state.mode(),
        Mode::AdsrEdit {
            module,
            parameter: 0
        }
    );

    state.apply(GuiAction::ValueUp);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().parameters()[2].value(),
        &ParameterValue::Float {
            value: 30,
            min: 0,
            max: 100,
            step: 5,
        }
    );

    state.apply(GuiAction::Down);
    state.apply(GuiAction::ValueDownFast);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().parameters()[3].value(),
        &ParameterValue::Float {
            value: 0,
            min: 0,
            max: 100,
            step: 5,
        }
    );
}

#[test]
fn envelope_special_editor_adds_moves_curves_and_deletes_points() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 1, 4);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Confirm);
    assert_eq!(
        state.mode(),
        Mode::EnvEdit {
            module,
            point: 0,
            editing: false,
        }
    );

    state.apply(GuiAction::AddPoint);
    assert_eq!(
        state
            .module_at(GridPos::new(0, 0))
            .unwrap()
            .env_points()
            .len(),
        3
    );
    assert_eq!(
        state.mode(),
        Mode::EnvEdit {
            module,
            point: 1,
            editing: false,
        }
    );

    state.apply(GuiAction::ToggleCurve);
    assert!(state.module_at(GridPos::new(0, 0)).unwrap().env_points()[1].curve);

    state.apply(GuiAction::Move);
    state.apply(GuiAction::Right);
    assert_eq!(
        state.mode(),
        Mode::EnvEdit {
            module,
            point: 1,
            editing: true,
        }
    );
    assert!(state.module_at(GridPos::new(0, 0)).unwrap().env_points()[1].time > 50);

    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::DeletePoint);
    assert_eq!(
        state
            .module_at(GridPos::new(0, 0))
            .unwrap()
            .env_points()
            .len(),
        2
    );
}

#[test]
fn probe_special_editor_changes_and_resets_probe_length() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 4, 0);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::ProbeEdit { module });

    state.apply(GuiAction::ValueUp);
    assert_eq!(state.probe_len(), 8820);
    state.apply(GuiAction::ValueDownFast);
    assert_eq!(state.probe_len(), 2205);
    state.apply(GuiAction::Delete);
    assert_eq!(state.probe_len(), 4410);
}

#[test]
fn sample_special_editor_zooms_pans_and_resets() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 6);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Confirm);
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 1,
            offset: 0,
        }
    );

    state.apply(GuiAction::ValueUp);
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 2,
            offset: 0,
        }
    );
    state.apply(GuiAction::Right);
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 2,
            offset: 5,
        }
    );
    state.apply(GuiAction::Delete);
    assert_eq!(
        state.mode(),
        Mode::SampleView {
            module,
            zoom: 1,
            offset: 0,
        }
    );
}

#[test]
fn aligned_modules_create_derived_connection() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Right);
    place_module(&mut state, 2, 0);

    let source = state.module_at(GridPos::new(0, 0)).unwrap().id();
    let target = state.module_at(GridPos::new(2, 0)).unwrap().id();
    let connections = state.connections();

    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].from(), source);
    assert_eq!(connections[0].to(), target);
}

#[test]
fn derived_connections_stop_at_nearest_module() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 2, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 7, 0);

    let source = state.module_at(GridPos::new(0, 0)).unwrap().id();
    let middle = state.module_at(GridPos::new(1, 0)).unwrap().id();
    let output = state.module_at(GridPos::new(2, 0)).unwrap().id();
    let connections = state.connections();

    assert_eq!(connections.len(), 2);
    assert!(
        connections
            .iter()
            .any(|connection| connection.from() == source && connection.to() == middle)
    );
    assert!(
        connections
            .iter()
            .any(|connection| connection.from() == middle && connection.to() == output)
    );
    assert!(
        !connections
            .iter()
            .any(|connection| connection.from() == source && connection.to() == output)
    );
}

#[test]
fn routing_modules_accept_derived_input_connections() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    state.apply(GuiAction::Right);
    place_module(&mut state, 5, 2);
    state.apply(GuiAction::Right);
    place_module(&mut state, 7, 0);

    let osc = state.module_at(GridPos::new(0, 0)).unwrap();
    let split = state.module_at(GridPos::new(1, 0)).unwrap();
    let output = state.module_at(GridPos::new(2, 0)).unwrap();
    assert_eq!(osc.kind(), ModuleKind::Osc);
    assert_eq!(split.kind(), ModuleKind::LeftSplit);
    assert_eq!(output.kind(), ModuleKind::Output);

    let connections = state.connections();
    assert_eq!(connections.len(), 2);
    assert!(
        connections
            .iter()
            .any(|connection| connection.from() == osc.id() && connection.to() == split.id())
    );
    assert!(
        connections
            .iter()
            .any(|connection| connection.from() == split.id() && connection.to() == output.id())
    );

    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    let mut audible = false;
    for _ in 0..128 {
        let frame = compiled.next();
        audible |= frame.left().value().abs() > 0.001 || frame.right().value().abs() > 0.001;
    }
    assert!(audible);
}

#[test]
fn haven_grid_renders_connections_and_ports() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Right);
    place_module(&mut state, 2, 0);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);

    assert_eq!(state.connections().len(), 1);
    assert!(pane.location(60_000).is_some());
    assert!(pane.location(60_001).is_none());
    assert!(pane.location(10_005).is_none());

    let grid = pane.location(990).unwrap();
    let left = grid.x - grid_width() * 0.5;
    let top = grid.y - grid_height() * 0.5;
    assert_point_near(
        pane.location(10_004).unwrap(),
        Point::new(left + 22.5, top + 15.),
    );
    assert_point_near(
        pane.location(40_020).unwrap(),
        Point::new(left + 73.5, top + 15.),
    );
}

#[test]
fn haven_grid_renders_multi_port_modules_as_multi_cell_tiles() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::Search);
    for character in "compressor".chars() {
        state.apply(GuiAction::InputChar(character));
    }
    state.apply(GuiAction::Confirm);

    let compressor = state.module_at(GridPos::new(0, 0)).unwrap().id();
    assert_eq!(state.module_at(GridPos::new(0, 5)).unwrap().id(), compressor);

    for _ in 0..3 {
        state.apply(GuiAction::Down);
    }
    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.modules().len(), 1);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);
    let first = pane.location(40_000).unwrap();
    let last = pane.location(40_005).unwrap();
    assert!(last.y > first.y + 150.);
    assert!(pane.location(10_004).is_some());
}

#[test]
fn haven_grid_renders_fixed_input_as_x_instead_of_port() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::ValueUp);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);

    assert!(pane.location(40_000).is_none());
    let grid = pane.location(990).unwrap();
    let left = grid.x - grid_width() * 0.5;
    let top = grid.y - grid_height() * 0.5;
    assert_point_near(pane.location(45_000).unwrap(), Point::new(left + 7.5, top + 15.));
}

#[test]
fn gui_built_osc_output_patch_emits_audio() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::CycleUnit);
    state.apply(GuiAction::CycleUnit);
    state.apply(GuiAction::CycleUnit);
    state.apply(GuiAction::TypeValue);
    state.apply(GuiAction::TextStart);
    state.apply(GuiAction::DeleteChar);
    state.apply(GuiAction::DeleteChar);
    state.apply(GuiAction::DeleteChar);
    state.apply(GuiAction::InputChar('2'));
    state.apply(GuiAction::InputChar('2'));
    state.apply(GuiAction::InputChar('0'));
    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::Cancel);
    state.apply(GuiAction::Right);
    place_module(&mut state, 7, 0);

    let osc = state.module_at(GridPos::new(0, 0)).unwrap();
    let output = state.module_at(GridPos::new(1, 0)).unwrap();
    assert_eq!(osc.kind(), ModuleKind::Osc);
    assert_eq!(output.kind(), ModuleKind::Output);
    assert!(
        state
            .connections()
            .iter()
            .any(|connection| connection.from() == osc.id() && connection.to() == output.id())
    );

    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();

    let mut audible = false;
    for _ in 0..128 {
        let frame = compiled.next();
        audible |= frame.left().value().abs() > 0.001 || frame.right().value().abs() > 0.001;
    }
    assert!(audible);
}

#[test]
fn gui_built_freq_osc_output_patch_uses_track_controls() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 4);
    state.apply(GuiAction::Right);
    place_module(&mut state, 7, 0);

    let freq = state.module_at(GridPos::new(0, 0)).unwrap();
    let osc = state.module_at(GridPos::new(1, 0)).unwrap();
    let output = state.module_at(GridPos::new(2, 0)).unwrap();
    assert_eq!(freq.kind(), ModuleKind::Freq);
    assert_eq!(osc.kind(), ModuleKind::Osc);
    assert_eq!(output.kind(), ModuleKind::Output);

    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    let controls = PatchControls {
        frequency: Hertz::new(330.0),
        gate: 1.0,
        degree: 2,
    };

    let mut audible = false;
    for _ in 0..128 {
        let frame = compiled.next_with_controls(controls);
        audible |= frame.left().value().abs() > 0.001 || frame.right().value().abs() > 0.001;
    }
    assert!(audible);
}

#[test]
fn track_edit_opens_note_prompt_and_commits_text() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::TrackEdit);
    assert_eq!(state.mode(), Mode::TrackPrompt);
    assert_eq!(state.prompt_text(), "(0/2/4/7)");

    state.apply(GuiAction::TextStart);
    for _ in 0..9 {
        state.apply(GuiAction::DeleteChar);
    }
    for character in "(0/_/7)".chars() {
        state.apply(GuiAction::InputChar(character));
    }
    state.apply(GuiAction::Confirm);

    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.track_text(), "(0/_/7)");
}

#[test]
fn selection_deletes_selected_modules() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 1);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 2);

    state.apply(GuiAction::Left);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Delete);

    assert!(state.module_at(GridPos::new(0, 0)).is_none());
    assert!(state.module_at(GridPos::new(1, 0)).is_none());
    assert!(state.module_at(GridPos::new(2, 0)).is_some());
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn selection_move_moves_group_and_cancel_exits_to_normal() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 1);

    state.apply(GuiAction::Left);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    assert_eq!(state.selected_modules().len(), 2);

    state.apply(GuiAction::Move);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Confirm);

    assert!(state.module_at(GridPos::new(1, 1)).is_some());
    assert!(state.module_at(GridPos::new(2, 1)).is_some());

    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Cancel);

    assert_eq!(state.mode(), Mode::Normal);
    assert!(state.module_at(GridPos::new(1, 1)).is_some());
    assert!(state.module_at(GridPos::new(2, 1)).is_some());
}

#[test]
fn selection_copy_places_group_and_keeps_originals() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 1);

    state.apply(GuiAction::Left);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Copy);
    assert_eq!(
        state.mode(),
        Mode::CopySelection {
            anchor: GridPos::new(0, 0),
            extent: GridPos::new(1, 0),
            origin: GridPos::new(1, 0)
        }
    );

    state.apply(GuiAction::Down);
    state.apply(GuiAction::Confirm);

    assert_eq!(state.modules().len(), 4);
    assert!(state.module_at(GridPos::new(0, 0)).is_some());
    assert!(state.module_at(GridPos::new(1, 0)).is_some());
    assert!(state.module_at(GridPos::new(0, 1)).is_some());
    assert!(state.module_at(GridPos::new(1, 1)).is_some());
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn haven_grid_drag_moves_module() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    pane.drag(
        &mut state,
        grid_cell_center(&pane, 0, 0),
        grid_cell_center(&pane, 2, 1),
    );

    assert!(state.module_at(GridPos::new(0, 0)).is_none());
    assert!(state.module_at(GridPos::new(2, 1)).is_some());
    assert_eq!(state.cursor(), GridPos::new(2, 1));
}

#[test]
fn haven_grid_drag_selects_empty_cells() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    pane.drag(
        &mut state,
        grid_cell_center(&pane, 0, 0),
        grid_cell_center(&pane, 2, 1),
    );

    assert_eq!(
        state.mode(),
        Mode::Select {
            anchor: GridPos::new(0, 0)
        }
    );
    assert_eq!(state.cursor(), GridPos::new(2, 1));
    assert_eq!(
        state.selection(),
        Some((GridPos::new(0, 0), GridPos::new(2, 1)))
    );
}

#[test]
fn haven_grid_drag_moves_selected_group() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 1);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    pane.drag(
        &mut state,
        grid_cell_center(&pane, 0, 0),
        grid_cell_center(&pane, 1, 1),
    );

    assert!(state.module_at(GridPos::new(0, 0)).is_none());
    assert!(state.module_at(GridPos::new(1, 0)).is_none());
    assert!(state.module_at(GridPos::new(1, 1)).is_some());
    assert!(state.module_at(GridPos::new(2, 1)).is_some());
    assert_eq!(
        state.selection(),
        Some((GridPos::new(1, 1), GridPos::new(2, 1)))
    );
}

#[test]
fn haven_grid_click_inside_selection_starts_selected_move() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    pane.click(&mut state, grid_cell_center(&pane, 0, 0));

    assert_eq!(
        state.mode(),
        Mode::SelectMove {
            anchor: GridPos::new(0, 0),
            extent: GridPos::new(1, 0),
            origin: GridPos::new(0, 0)
        }
    );
}

#[test]
fn haven_grid_click_outside_selection_clears_selection() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    pane.click(&mut state, grid_cell_center(&pane, 3, 0));

    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.cursor(), GridPos::new(3, 0));
    assert_eq!(state.selection(), None);
}

#[test]
fn haven_grid_click_focuses_cells_without_starting_modes() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    pane.click(&mut state, grid_cell_center(&pane, 3, 2));
    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.cursor(), GridPos::new(3, 2));
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().id(), module);

    pane.click(&mut state, grid_cell_center(&pane, 0, 0));
    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.cursor(), GridPos::new(0, 0));
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().id(), module);
}

#[test]
fn undo_redo_restore_patch_mutations() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    assert_eq!(state.modules().len(), 1);

    state.apply(GuiAction::Undo);
    assert!(state.modules().is_empty());

    state.apply(GuiAction::Redo);
    assert_eq!(state.modules().len(), 1);
}

#[test]
fn instruments_are_separate_patch_surfaces() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);

    state.apply(GuiAction::NewInstrument);
    assert_eq!(state.active_instrument(), 1);
    assert_eq!(state.instrument_count(), 2);
    assert!(state.modules().is_empty());

    place_module(&mut state, 0, 1);
    state.apply(GuiAction::Instrument(0));
    assert_eq!(state.active_instrument(), 0);
    assert_eq!(state.modules().len(), 1);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Freq
    );

    state.apply(GuiAction::Instrument(1));
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );
}

#[test]
fn global_toggles_and_help_scroll_match_normal_mode_actions() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::TogglePlay);
    state.apply(GuiAction::ToggleMeters);
    state.apply(GuiAction::HelpScrollDown);
    state.apply(GuiAction::HelpScrollDown);
    state.apply(GuiAction::HelpScrollUp);

    assert!(state.playing());
    assert!(state.show_meters());
    assert_eq!(state.help_scroll(), 1);
}

#[test]
fn load_action_sets_one_shot_load_request() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::Load);

    assert!(state.take_load_request());
    assert!(!state.take_load_request());
}

#[test]
fn load_project_replaces_state_from_project_file() {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "brainwash-gui-load-{}-{}.bw",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    fs::write(
        &path,
        r#"(
  bpm: 132.0,
  bars: 1.0,
  scale_idx: 2,
  modules: [
    (
      id: 7,
      kind: Standard(Osc),
      x: 1,
      y: 2,
      params: Osc(
        wave: Squ,
        freq: (unit: Hz, seconds: 0.00227, samples: 100.0, bar_num: 1, bar_denom: 4, hz: 440.0),
        shift: 0.0,
        gain: 0.75,
        uni: false,
        connected: 255,
      ),
    ),
    (
      id: 8,
      kind: Standard(Output),
      x: 3,
      y: 2,
      params: Output(
        gain: 0.5,
        connected: 255,
      ),
    ),
  ],
  track: Some("(0/4)"),
  subpatches: [],
)"#,
    )
    .unwrap();

    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::Confirm);
    state.load_project(&path).unwrap();

    assert_eq!(state.saved_path(), Some(path.to_string_lossy().as_ref()));
    assert_eq!(state.bpm(), 132);
    assert_eq!(state.scale_index(), 2);
    assert_eq!(state.track_text(), "(0/4)");
    assert!(!state.dirty());
    assert_eq!(state.modules().len(), 2);

    let osc = state
        .modules()
        .iter()
        .find(|module| module.kind() == ModuleKind::Osc)
        .unwrap();
    assert_eq!(osc.position(), GridPos::new(1, 2));
    assert_eq!(osc.parameters()[0].value_label(), "square");
    assert_eq!(osc.parameters()[1].value_label(), "440 hz");
    assert_eq!(osc.parameters()[3].value_label(), "0.75");

    let output = state
        .modules()
        .iter()
        .find(|module| module.kind() == ModuleKind::Output)
        .unwrap();
    assert_eq!(output.position(), GridPos::new(3, 2));
    assert_eq!(output.parameters()[1].value_label(), "0.50");
    assert!(!state.take_load_request());

    let _ = fs::remove_file(path);
}

fn place_module(state: &mut GuiState, category_rights: usize, selection_downs: usize) {
    state.apply(GuiAction::OpenPalette);
    for _ in 0..category_rights {
        state.apply(GuiAction::PaletteRight);
    }
    for _ in 0..selection_downs {
        state.apply(GuiAction::PaletteDown);
    }
    state.apply(GuiAction::Confirm);
}

fn grid_cell_center(pane: &haven::Pane<GuiState>, x: u16, y: u16) -> Point {
    const CELL: f32 = 30.;
    const GAP: f32 = 3.;
    let grid = pane.location(990).unwrap();
    let left = grid.x - grid_width() * 0.5;
    let top = grid.y - grid_height() * 0.5;
    Point::new(
        left + x as f32 * (CELL + GAP) + CELL * 0.5,
        top + y as f32 * (CELL + GAP) + CELL * 0.5,
    )
}

fn grid_width() -> f32 {
    const CELL: f32 = 30.;
    const GAP: f32 = 3.;
    const COLUMNS: f32 = 16.;
    COLUMNS * CELL + (COLUMNS - 1.) * GAP
}

fn grid_height() -> f32 {
    const CELL: f32 = 30.;
    const GAP: f32 = 3.;
    const ROWS: f32 = 12.;
    ROWS * CELL + (ROWS - 1.) * GAP
}

fn assert_point_near(actual: Point, expected: Point) {
    let dx = (actual.x - expected.x).abs();
    let dy = (actual.y - expected.y).abs();
    assert!(dx < 0.5 && dy < 0.5, "{actual:?} != {expected:?}");
}

fn assert_point_near_x(actual: Point, expected: f32) {
    assert!((actual.x - expected).abs() < 0.5, "{actual:?} != {expected}");
}

fn assert_point_near_y(actual: Point, expected: f32) {
    assert!((actual.y - expected).abs() < 0.5, "{actual:?} != {expected}");
}
