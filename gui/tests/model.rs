use brainwash::compile::PatchControls;
use brainwash::time::{Hertz, SampleRate};
use brainwash_gui::model::{
    AudioPatchError, GridPos, GuiAction, GuiState, Mode, ModuleCategory, ModuleKind, Orientation,
    ParameterValue, all_modules,
};
use brainwash_gui::view::main_view;
use haven::{
    Key, MouseButton, NamedKey, Pane, PaneBuilder, Point, render::Frame, render::RenderItem,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn module_inventory_has_expected_surface_count() {
    assert_eq!(all_modules().len(), 48);
    assert_eq!(all_modules()[0], ModuleKind::Phase);
    assert_eq!(all_modules()[1], ModuleKind::Output);
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
fn default_phase_is_a_primitive_unit() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::Confirm);

    let module = state.module_at(GridPos::new(0, 0)).unwrap();
    assert_eq!(module.kind(), ModuleKind::Phase);
    assert_eq!(module.label(), "Phase");
    assert_eq!(module.parameters().len(), 1);

    state.apply(GuiAction::EditComposition);
    assert_eq!(state.composition_depth(), 0);
}

#[test]
fn phase_has_no_inner_surface() {
    let mut state = GuiState::new(16, 16);
    place_module_kind(&mut state, ModuleKind::Phase);
    state.apply(GuiAction::EditComposition);
    assert_eq!(state.composition_depth(), 0);
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
fn fast_movement_uses_four_cell_step() {
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
    assert_eq!(module.kind(), ModuleKind::Freq);
    assert_eq!(module.orientation(), Orientation::Right);
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn palette_category_changes_open_the_remembered_selection() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::PaletteRight);

    assert_eq!(state.palette_category(), ModuleCategory::Output);
    assert_eq!(state.selected_palette_module(), ModuleKind::Output);
}

#[test]
fn direct_palette_category_preserves_each_category_selection() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::Palette(ModuleCategory::Effect));
    state.apply(GuiAction::PaletteDown);
    assert_eq!(state.mode(), Mode::Palette);
    assert_eq!(state.palette_category(), ModuleCategory::Effect);

    state.apply(GuiAction::Palette(ModuleCategory::Filter));
    assert_eq!(state.palette_category(), ModuleCategory::Filter);
    assert_eq!(state.selected_palette_module(), ModuleKind::Filter);

    state.apply(GuiAction::Palette(ModuleCategory::Effect));
    assert_eq!(state.selected_palette_module(), ModuleKind::Comb);

    state.apply(GuiAction::Cancel);
    state.apply(GuiAction::OpenPalette);
    assert_eq!(state.palette_category(), ModuleCategory::Effect);
    assert_eq!(state.selected_palette_module(), ModuleKind::Comb);
}

#[test]
fn palette_search_filters_and_places_module() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::Search);
    for character in "filter".chars() {
        state.apply(GuiAction::InputChar(character));
    }

    assert!(state.palette_searching());
    assert_eq!(state.palette_filter(), "filter");
    assert_eq!(
        state.selected_filtered_palette_module(),
        Some(ModuleKind::Filter)
    );

    state.apply(GuiAction::Confirm);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Primitive
    );
    assert_eq!(state.mode(), Mode::Normal);
    assert!(!state.palette_searching());
}

#[test]
fn palette_search_navigation_backspace_and_cancel_are_consistent() {
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

    for character in "filter".chars() {
        pane.key_pressed(&mut state, Key::character(character.to_string()));
    }
    assert_eq!(
        state.selected_filtered_palette_module(),
        Some(ModuleKind::Filter)
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
fn move_confirm_cancel_delete_and_rotate_work() {
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
fn move_places_multi_cell_module_relative_to_grabbed_cell() {
    let mut state = GuiState::default();
    place_palette_module(&mut state, ModuleCategory::Shape, "ADSR");
    let id = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Down);
    assert_eq!(state.module_at(GridPos::new(0, 1)).unwrap().id(), id);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Confirm);

    assert!(state.module_at(GridPos::new(0, 0)).is_none());
    assert_eq!(state.module_at(GridPos::new(1, 0)).unwrap().id(), id);
    assert_eq!(state.module_at(GridPos::new(1, 1)).unwrap().id(), id);
}

#[test]
fn copy_can_overlap_and_disables_overlapping_modules() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    let source = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Copy);
    assert_eq!(state.mode(), Mode::Copy { source });
    state.apply(GuiAction::Confirm);
    assert_eq!(state.modules().len(), 2);
    assert_eq!(
        state
            .modules()
            .iter()
            .filter(|module| module.disabled())
            .count(),
        2
    );
    assert_eq!(state.mode(), Mode::Normal);
}

#[test]
fn copy_places_module_at_empty_target() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    let source = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Copy);
    assert_eq!(state.mode(), Mode::Copy { source });
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Confirm);

    assert_eq!(state.modules().len(), 2);
    assert_eq!(
        state
            .modules()
            .iter()
            .filter(|module| module.disabled())
            .count(),
        0
    );
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
fn editing_a_composition_changes_its_name() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    assert_eq!(
        state.mode(),
        Mode::Edit {
            module,
            parameter: 0
        }
    );

    state.apply(GuiAction::TypeValue);
    assert_eq!(state.prompt_text(), "Composition");
    state.apply(GuiAction::Cancel);
    assert_eq!(
        state.mode(),
        Mode::Edit {
            module,
            parameter: 0
        }
    );
}

#[test]
fn fraction_time_parameters_use_fraction_scale() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 3, 0);

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::CycleUnit);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().parameters()[1].value_label(),
        "1/4"
    );

    state.apply(GuiAction::ValueUp);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().parameters()[1].value_label(),
        "2/4"
    );
}

#[test]
fn typed_value_input_renames_a_composition() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::TypeValue);
    assert_eq!(
        state.mode(),
        Mode::ValueInput {
            module,
            parameter: 0
        }
    );

    state.apply(GuiAction::TextStart);
    for _ in 0..11 {
        state.apply(GuiAction::DeleteChar);
    }
    for character in "Tone".chars() {
        state.apply(GuiAction::InputChar(character));
    }
    state.apply(GuiAction::Confirm);

    assert_eq!(
        state.mode(),
        Mode::Edit {
            module,
            parameter: 0
        }
    );
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().label(), "Tone");
}

#[test]
fn composition_name_input_accepts_text() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::TypeValue);
    assert_eq!(
        state.mode(),
        Mode::ValueInput {
            module,
            parameter: 0
        }
    );
    assert_eq!(state.prompt_text(), "Composition");
    state.apply(GuiAction::InputChar('/'));
    state.apply(GuiAction::InputChar('a'));
    assert_eq!(state.prompt_text(), "Composition/a");
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
    assert!(state.take_save_request());
    let save_path = project_path("save-prompt");
    assert!(state.save_project(&save_path));
    assert_eq!(
        state.saved_path(),
        Some(save_path.to_string_lossy().as_ref())
    );
    assert!(!state.dirty());
    let project = brainwash_grid::project::load(&save_path).unwrap();
    assert_eq!(project.modules.len(), 1);
    assert_eq!(
        project.modules[0].kind,
        brainwash_grid::project::ModuleKind::Standard(
            brainwash_grid::project::StandardModule::Freq
        )
    );

    state.apply(GuiAction::Export);
    assert_eq!(state.mode(), Mode::ExportPrompt);
    state.apply(GuiAction::Up);
    assert_eq!(state.export_loops(), 2);
    state.apply(GuiAction::Confirm);
    assert!(state.exported_path().unwrap().ends_with(".wav"));
}

#[test]
fn save_and_export_prompt_confirm_existing_paths_before_overwrite() {
    let mut state = GuiState::new(8, 8);
    let save_path = project_path("overwrite-save");
    let export_path = project_path("overwrite-export").with_extension("wav");

    state.apply(GuiAction::Save);
    assert!(state.take_save_request());
    assert!(state.save_project(&save_path));
    assert_eq!(
        state.saved_path(),
        Some(save_path.to_string_lossy().as_ref())
    );

    state.apply(GuiAction::Export);
    replace_prompt_text(&mut state, &export_path.to_string_lossy());
    state.apply(GuiAction::Confirm);
    assert_eq!(
        state.exported_path(),
        Some(export_path.to_string_lossy().as_ref())
    );

    state.apply(GuiAction::SaveAs);
    assert!(state.take_save_request());
    assert!(state.save_project(&export_path));
    assert_eq!(
        state.saved_path(),
        Some(export_path.to_string_lossy().as_ref())
    );

    state.apply(GuiAction::SaveAs);
    assert!(state.take_save_request());
    assert!(state.save_project(&save_path));
    assert_eq!(
        state.saved_path(),
        Some(save_path.to_string_lossy().as_ref())
    );

    state.apply(GuiAction::Export);
    replace_prompt_text(&mut state, &save_path.to_string_lossy());
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::ExportConfirm);
    state.apply(GuiAction::Cancel);
    assert_eq!(state.mode(), Mode::ExportPrompt);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::ExportConfirm);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(
        state.exported_path(),
        Some(save_path.to_string_lossy().as_ref())
    );
}

#[test]
fn saving_existing_file_requires_overwrite_or_save_as_choice() {
    let mut state = GuiState::new(8, 8);
    let save_path = project_path("save-confirm");

    state.apply(GuiAction::Save);
    assert!(state.take_save_request());
    assert!(state.save_project(&save_path));

    state.apply(GuiAction::Save);
    assert_eq!(state.mode(), Mode::SaveConfirm);
    assert!(!state.take_save_request());

    state.apply(GuiAction::Cancel);
    assert_eq!(state.mode(), Mode::Normal);
    assert!(!state.take_save_request());

    state.apply(GuiAction::Save);
    state.apply(GuiAction::SaveAs);
    assert_eq!(state.mode(), Mode::Normal);
    assert!(state.take_save_request());

    state.apply(GuiAction::Save);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.mode(), Mode::Normal);
    assert!(!state.dirty());

    let _ = fs::remove_file(save_path);
}

#[test]
fn track_settings_apply_bounds_and_wrapping() {
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
fn composition_navigation_uses_isolated_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    let composition = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::EditComposition);
    assert_eq!(state.composition_depth(), 1);
    assert_eq!(state.cursor(), GridPos::new(0, 0));
    assert!(state.modules().is_empty());

    place_module_kind(&mut state, ModuleKind::Gate);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );

    state.apply(GuiAction::ExitComposition);
    assert_eq!(state.composition_depth(), 0);
    assert_eq!(state.cursor(), GridPos::new(0, 0));
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Composition
    );

    state.apply(GuiAction::EditComposition);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );
    assert_ne!(
        state.module_at(GridPos::new(0, 0)).unwrap().id(),
        composition
    );
}

#[test]
fn copying_composition_clones_its_surface_with_fresh_module_ids() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::Gate);
    let original_child = state.module_at(GridPos::new(0, 0)).unwrap().id();

    state.apply(GuiAction::ExitComposition);
    state.apply(GuiAction::Copy);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Confirm);

    state.apply(GuiAction::EditComposition);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );
    assert_ne!(
        state.module_at(GridPos::new(0, 0)).unwrap().id(),
        original_child
    );
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Freq);

    state.apply(GuiAction::ExitComposition);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::EditComposition);
    assert!(state.module_at(GridPos::new(1, 0)).is_none());
}

#[test]
fn nested_compositions_round_trip_through_project_files() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::ExitComposition);
    state.apply(GuiAction::ExitComposition);

    let path = project_path("nested-composition");
    assert!(state.save_project(&path));

    let mut loaded = GuiState::new(8, 8);
    loaded.load_project(&path).unwrap();
    loaded.apply(GuiAction::EditComposition);
    assert_eq!(
        loaded.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );
    loaded.apply(GuiAction::Right);
    loaded.apply(GuiAction::EditComposition);
    assert_eq!(
        loaded.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Freq
    );

    let _ = fs::remove_file(path);
}

#[test]
fn palette_color_category_survives_project_round_trip() {
    let mut state = GuiState::new(32, 24);
    place_palette_module(&mut state, ModuleCategory::Effect, "Reverb");
    let first = project_path("effect-color-first");
    let second = project_path("effect-color-second");
    assert!(state.save_project(&first));

    let mut loaded = GuiState::new(32, 24);
    loaded.load_project(&first).unwrap();
    assert!(loaded.save_project(&second));
    let project = brainwash_grid::project::load(&second).unwrap();
    let brainwash_grid::project::ModuleParams::Composition { color, .. } =
        &project.modules[0].params
    else {
        panic!("expected composition parameters");
    };
    assert_eq!(*color, (154, 86, 178));

    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[test]
fn composition_inputs_are_editable_instance_parameters_and_persist() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::CompositionInput);
    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::TypeValue);
    replace_prompt_text(&mut state, "0.25");
    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::Cancel);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::CompositionOutput);
    state.apply(GuiAction::ExitComposition);

    let composition = state.module_at(GridPos::new(0, 0)).unwrap();
    assert_eq!(composition.parameters().len(), 2);
    assert_eq!(composition.parameters()[1].name(), "Input");
    assert_eq!(composition.parameters()[1].value_label(), "0.25");
    assert!(!composition.parameters()[1].connected());

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::ValueUp);
    state.apply(GuiAction::Cancel);
    let edited = state.modules()[0].parameters()[1].value_label();
    assert_ne!(edited, "0.25");

    let path = project_path("composition-input-params");
    assert!(state.save_project(&path));
    let mut loaded = GuiState::new(8, 8);
    loaded.load_project(&path).unwrap();
    let input = &loaded.modules()[0].parameters()[1];
    assert_eq!(input.value_label(), edited);
    assert!(!input.connected());

    let _ = fs::remove_file(path);
}

#[test]
fn unwired_composition_input_uses_its_edited_value() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::CompositionInput);
    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::TypeValue);
    replace_prompt_text(&mut state, "0.25");
    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::TogglePort);
    state.apply(GuiAction::Cancel);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::CompositionOutput);
    state.apply(GuiAction::ExitComposition);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Output);

    let mut patch = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    assert!((patch.next().left().value() - 0.25).abs() < 0.001);
}

#[test]
fn moving_module_can_enter_composition_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Gate);
    let module = state.module_at(GridPos::new(1, 0)).unwrap().id();

    state.apply(GuiAction::Move);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::EditComposition);
    assert_eq!(state.composition_depth(), 1);
    assert!(state.modules().is_empty());

    state.apply(GuiAction::Confirm);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Gate
    );
    state.apply(GuiAction::ExitComposition);
    assert!(state.module_at(GridPos::new(1, 0)).is_none());
    state.apply(GuiAction::EditComposition);
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().id(), module);
}

#[test]
fn cancelling_composition_move_restores_module_to_origin_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Gate);
    let module = state.module_at(GridPos::new(1, 0)).unwrap().id();

    state.apply(GuiAction::Move);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::EditComposition);
    state.apply(GuiAction::Cancel);

    assert_eq!(state.composition_depth(), 0);
    assert_eq!(state.cursor(), GridPos::new(1, 0));
    assert_eq!(state.module_at(GridPos::new(1, 0)).unwrap().id(), module);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::EditComposition);
    assert!(state.modules().is_empty());
}

#[test]
fn moving_selection_can_enter_composition_surface() {
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
    state.apply(GuiAction::EditComposition);
    assert_eq!(state.composition_depth(), 1);
    assert!(state.modules().is_empty());

    state.apply(GuiAction::Right);
    state.apply(GuiAction::Confirm);
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().id(), first);
    assert_eq!(state.module_at(GridPos::new(1, 0)).unwrap().id(), second);
    state.apply(GuiAction::ExitComposition);
    assert!(state.module_at(GridPos::new(1, 0)).is_none());
    assert!(state.module_at(GridPos::new(2, 0)).is_none());
}

#[test]
fn cancelling_composition_selection_move_restores_origin_surface() {
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
    state.apply(GuiAction::EditComposition);
    state.apply(GuiAction::Cancel);

    assert_eq!(state.composition_depth(), 0);
    assert_eq!(state.cursor(), GridPos::new(2, 0));
    assert_eq!(state.module_at(GridPos::new(1, 0)).unwrap().id(), first);
    assert_eq!(state.module_at(GridPos::new(2, 0)).unwrap().id(), second);
    state.apply(GuiAction::LeftFast);
    state.apply(GuiAction::EditComposition);
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
fn haven_transient_mode_confirm_keys_work() {
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
        ModuleKind::Freq
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
fn haven_native_save_requests_match_gui_actions() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);
    let save_path = project_path("haven-overwrite-save");
    let export_path = project_path("haven-overwrite-export").with_extension("wav");

    pane.key_pressed(&mut state, Key::character("w"));
    assert!(state.take_save_request());
    assert!(state.save_project(&save_path));
    assert_eq!(
        state.saved_path(),
        Some(save_path.to_string_lossy().as_ref())
    );

    pane.key_pressed(&mut state, Key::character("e"));
    replace_prompt_text_pane(&mut pane, &mut state, &export_path.to_string_lossy());
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(
        state.exported_path(),
        Some(export_path.to_string_lossy().as_ref())
    );

    pane.key_pressed(&mut state, Key::character("W"));
    assert!(state.take_save_request());
    assert!(state.save_project(&export_path));
    assert_eq!(
        state.saved_path(),
        Some(export_path.to_string_lossy().as_ref())
    );
}

#[test]
fn haven_palette_search_keys_drive_model() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("n"));
    pane.key_pressed(&mut state, Key::character("/"));
    for character in "filter".chars() {
        pane.key_pressed(&mut state, Key::character(character.to_string()));
    }
    pane.key_pressed(&mut state, NamedKey::Enter);

    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Primitive
    );
}

#[test]
fn haven_palette_category_keys_work() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("@"));
    assert_eq!(state.mode(), Mode::Palette);
    assert_eq!(state.palette_category(), ModuleCategory::Shape);
    assert_eq!(state.selected_palette_label(), "Envelope");
    assert_eq!(state.selected_palette_module(), ModuleKind::Envelope);

    pane.key_pressed(&mut state, Key::character("#"));
    assert_eq!(state.palette_category(), ModuleCategory::Filter);
    assert_eq!(state.selected_palette_module(), ModuleKind::Filter);

    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(
        state.module_at(GridPos::new(0, 0)).unwrap().kind(),
        ModuleKind::Primitive
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
fn haven_save_module_button_only_renders_inside_a_composition() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);
    let save = pane.location(3_060).unwrap();
    let export = pane.location(3_062).unwrap();
    pane.click(&mut state, Point::new((save.x + export.x) * 0.5, save.y));
    assert!(state.take_save_module_request().is_none());

    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    pane.redraw(&mut state, 760, 560, 1.0);
    let save = pane.location(3_060).unwrap();
    let export = pane.location(3_062).unwrap();
    pane.click(&mut state, Point::new((save.x + export.x) * 0.5, save.y));
    assert!(state.take_save_module_request().is_some());
}

#[test]
fn haven_palette_background_stays_top_aligned_when_switching_tabs() {
    let panel_area = |pane: &Pane<GuiState>, frame: &Frame| {
        let center = pane.location(200).unwrap();
        frame
            .items
            .iter()
            .find_map(|item| match item {
                RenderItem::Path { area, .. }
                    if (area.x + area.width * 0.5 - center.x).abs() < 0.5
                        && (area.y + area.height * 0.5 - center.y).abs() < 0.5 =>
                {
                    Some(*area)
                }
                _ => None,
            })
            .unwrap()
    };

    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::OpenPalette);
    let mut pane = PaneBuilder::new("main", main_view).build();

    for category in ModuleCategory::ALL {
        state.apply(GuiAction::Palette(category));
        let (frame, _) = pane.redraw(&mut state, 760, 560, 1.0);
        let area = panel_area(&pane, &frame);
        assert!((area.y - 40.).abs() < 0.5, "{category:?}: {area:?}");
    }

    state.apply(GuiAction::Search);
    for character in "lpf".chars() {
        state.apply(GuiAction::InputChar(character));
    }
    let (frame, _) = pane.redraw(&mut state, 760, 560, 1.0);
    let area = panel_area(&pane, &frame);
    assert!((area.y - 40.).abs() < 0.5, "{area:?}");
}

#[test]
fn haven_routing_palette_draws_a_leading_icon_for_every_module() {
    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::Palette(ModuleCategory::Routing));
    let mut pane = PaneBuilder::new("main", main_view).build();
    let (frame, _) = pane.redraw(&mut state, 760, 560, 1.0);
    let icons = frame
        .items
        .iter()
        .filter(|item| match item {
            RenderItem::Path { area, .. } => {
                (area.width - 17.).abs() < 0.5 && (area.height - 17.).abs() < 0.5
            }
            _ => false,
        })
        .count();
    assert_eq!(icons, 6);
}

#[test]
fn haven_prompt_keys_drive_model() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("w"));
    assert!(state.take_save_request());
    let save_path = project_path("haven-prompt");
    assert!(state.save_project(&save_path));
    assert_eq!(
        state.saved_path(),
        Some(save_path.to_string_lossy().as_ref())
    );

    pane.key_pressed(&mut state, Key::character("e"));
    pane.key_pressed(&mut state, NamedKey::ArrowUp);
    pane.key_pressed(&mut state, NamedKey::Enter);
    assert!(state.exported_path().unwrap().ends_with(".wav"));
    assert_eq!(state.export_loops(), 2);

    pane.key_pressed(&mut state, Key::character("Q"));
    assert!(state.should_quit());
}

#[test]
fn haven_prompt_editing_keys_work() {
    let mut state = GuiState::new(8, 8);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("w"));
    assert!(state.take_save_request());
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
            parameter: 0
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
fn haven_composition_key_enters_and_exits_surface() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("p"));
    assert_eq!(state.composition_depth(), 1);

    pane.key_pressed(&mut state, Key::character("p"));
    assert_eq!(state.composition_depth(), 0);
}

#[test]
fn haven_move_key_can_place_module_inside_composition() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 6, 2);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Gate);
    let module = state.module_at(GridPos::new(1, 0)).unwrap().id();
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    pane.key_pressed(&mut state, Key::character("m"));
    pane.key_pressed(&mut state, Key::character("h"));
    pane.key_pressed(&mut state, Key::character("p"));
    assert_eq!(state.composition_depth(), 1);

    pane.key_pressed(&mut state, NamedKey::Enter);
    assert_eq!(state.module_at(GridPos::new(0, 0)).unwrap().id(), module);
}

#[test]
fn haven_selection_move_key_can_place_group_inside_composition() {
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
    assert_eq!(state.composition_depth(), 1);

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
            parameter: 0,
        }
    );
    assert!(matches!(
        state.module_at(GridPos::new(0, 0)).unwrap().parameters()[0].value(),
        ParameterValue::Float { .. }
    ));
    pane.key_pressed(&mut state, NamedKey::Space);
    assert!(state.playing());
    assert_eq!(
        state.mode(),
        Mode::Edit {
            module,
            parameter: 0,
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
    assert!(pane.location(69_900).is_some());
    assert!(pane.location(301).is_none());
    let panel = pane.location(300).unwrap();
    let first_row = pane.location(600).unwrap();
    assert_point_near_x(first_row, panel.x);
    assert!(pane.location(3_048).is_none());
    assert!(pane.location(3_052).is_none());
    assert!(pane.location(3_055).is_none());
}

#[test]
fn haven_visual_editors_redraw() {
    let mut pane = PaneBuilder::new("main", main_view).build();

    let mut adsr = GuiState::new(8, 8);
    place_palette_module(&mut adsr, ModuleCategory::Shape, "ADSR");
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
    assert!(pane.location(843).is_some());
    assert!(pane.location(841).is_some());
    assert!(pane.location(880).is_some());
    assert!(pane.location(881).is_some());

    let mut probe = GuiState::new(8, 8);
    place_module(&mut probe, 4, 0);
    probe.apply(GuiAction::Edit);
    probe.apply(GuiAction::Down);
    probe.apply(GuiAction::Confirm);
    pane.redraw(&mut probe, 920, 720, 1.0);
    assert!(pane.location(901).is_some());
    assert!(pane.location(902).is_some());
    assert!(pane.location(903).is_some());
    assert!(pane.location(904).is_some());
    assert!(pane.location(905).is_some());
    assert!(pane.location(906).is_some());

    let mut sample = GuiState::new(8, 8);
    place_module(&mut sample, 0, 6);
    sample.apply(GuiAction::Edit);
    sample.apply(GuiAction::Down);
    sample.apply(GuiAction::Down);
    sample.apply(GuiAction::Confirm);
    pane.redraw(&mut sample, 920, 720, 1.0);
}

#[test]
fn haven_envelope_move_keys_work() {
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
fn haven_envelope_points_drag_with_mouse() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 1, 4);
    let module = state.module_at(GridPos::new(0, 0)).unwrap().id();
    let mut pane = PaneBuilder::new("main", main_view).build();

    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Confirm);
    pane.redraw(&mut state, 920, 720, 1.0);

    let target = pane.location(842).unwrap();
    let start = Point::new(target.x - 60., target.y);
    pane.move_to(&mut state, start);
    pane.press_button(&mut state, MouseButton::Left);
    pane.move_to(&mut state, target);
    pane.release_button(&mut state, MouseButton::Left);

    assert_eq!(
        state.mode(),
        Mode::EnvEdit {
            module,
            point: 0,
            editing: false,
        }
    );
    let points = state.module_at(GridPos::new(0, 0)).unwrap().env_points();
    assert!(
        points
            .iter()
            .any(|point| point.time >= 45 && point.time <= 55 && point.value.abs() <= 5),
        "{points:?}"
    );
}

#[test]
fn haven_probe_keys_work() {
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
fn haven_sample_keys_work() {
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
fn adsr_opens_as_a_composition() {
    let mut state = GuiState::default();
    place_palette_module(&mut state, ModuleCategory::Shape, "ADSR");
    state.apply(GuiAction::EditComposition);
    assert_eq!(state.composition_depth(), 1);
    assert!(!state.modules().is_empty());
}

#[test]
fn attenuator_is_available_as_an_editable_composition() {
    let mut state = GuiState::default();
    place_palette_module(&mut state, ModuleCategory::Effect, "Attenuator");
    state.apply(GuiAction::EditComposition);
    assert_eq!(state.composition_depth(), 1);
    assert!(!state.modules().is_empty());
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

    state.apply(GuiAction::ToggleCurve);
    assert!(!state.module_at(GridPos::new(0, 0)).unwrap().env_points()[0].curve);

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
fn bipolar_envelope_compiles_and_emits_negative_values() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Envelope);
    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::DownFast);
    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::Cancel);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Output);

    let mut patch = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    assert!((patch.next().left().value() + 1.0).abs() < 0.001);
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
fn built_in_composition_inputs_create_derived_connections() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Phase);

    assert_eq!(state.connections().len(), 1);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);

    assert!(pane.location(60_000).is_some());
}

#[test]
fn derived_connections_stop_at_nearest_module() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 2, 0);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Down);
    place_module(&mut state, 7, 0);

    let source = state.module_at(GridPos::new(0, 0)).unwrap().id();
    let middle = state.module_at(GridPos::new(1, 0)).unwrap().id();
    let output = state.module_at(GridPos::new(2, 2)).unwrap().id();
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
    assert_eq!(osc.kind(), ModuleKind::Phase);
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
fn routing_join_compiles_with_both_inputs_connected() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::TurnRightDown);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::RightJoin);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Output);

    let connections = state.connections();
    assert_eq!(connections.len(), 4);
    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    let frame = compiled.next_with_controls(PatchControls {
        frequency: Hertz::new(330.0),
        gate: 1.0,
        degree: 0,
    });

    assert!(frame.left().value() > 0.0);
}

#[test]
fn filter_frequency_port_compiles_from_semantic_shape() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Phase);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Filter);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Up);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Output);

    state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
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

    let origin = grid_cell_origin(&pane, GridPos::new(0, 0));
    assert_point_near(
        pane.location(10_004).unwrap(),
        Point::new(origin.x + 22.5, origin.y + 15.),
    );
    assert_point_near(
        pane.location(40_020).unwrap(),
        Point::new(origin.x + 73.5, origin.y + 15.),
    );
}

#[test]
fn haven_grid_renders_cursor_over_module() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Phase);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    let cell = pane.location(view_cell_id(GridPos::new(0, 0))).unwrap();
    let cursor = pane.location(72_000).unwrap();

    assert_point_near(cursor, cell);
}

#[test]
fn haven_grid_renders_routing_ports_on_physical_edges() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::TopSplit);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);

    let origin = grid_cell_origin(&pane, GridPos::new(0, 0));
    assert_point_near(
        pane.location(41_000).unwrap(),
        Point::new(origin.x + 15., origin.y + 7.5),
    );
    assert_point_near(
        pane.location(43_000).unwrap(),
        Point::new(origin.x + 15., origin.y + 22.5),
    );
    assert_point_near(
        pane.location(42_001).unwrap(),
        Point::new(origin.x + 22.5, origin.y + 15.),
    );
    assert!(pane.location(42_000).is_none());
    assert!(pane.location(43_001).is_none());
}

#[test]
fn haven_grid_renders_composition_input_port() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);
    assert!(pane.location(40_000).is_none());
    assert!(pane.location(10_004).is_none());

    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::CompositionInput);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::CompositionOutput);
    state.apply(GuiAction::ExitComposition);
    pane.redraw(&mut state, 760, 560, 1.0);

    let origin = grid_cell_origin(&pane, GridPos::new(0, 0));
    assert_point_near(
        pane.location(40_000).unwrap(),
        Point::new(origin.x + 7.5, origin.y + 15.),
    );
    assert_point_near(
        pane.location(10_004).unwrap(),
        Point::new(origin.x + 22.5, origin.y + 15.),
    );
}

#[test]
fn haven_grid_renders_multi_port_modules_as_multi_cell_tiles() {
    let mut state = GuiState::default();
    state.apply(GuiAction::OpenPalette);
    state.apply(GuiAction::Search);
    for character in "compressor".chars() {
        state.apply(GuiAction::InputChar(character));
    }
    state.apply(GuiAction::Confirm);

    let compressor = state.module_at(GridPos::new(0, 0)).unwrap().id();
    assert_eq!(
        state.module_at(GridPos::new(0, 5)).unwrap().id(),
        compressor
    );

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
    state.apply(GuiAction::Cancel);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 760, 560, 1.0);

    assert!(pane.location(40_000).is_none());
}

#[test]
fn gui_built_osc_output_patch_emits_audio() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 4);
    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
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
    assert_eq!(osc.kind(), ModuleKind::Phase);
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
fn gui_composition_routes_parent_input_to_child_composition_input() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::CompositionInput);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Phase);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::CompositionOutput);
    state.apply(GuiAction::ExitComposition);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Output);
    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    let controls = PatchControls {
        frequency: Hertz::new(330.0),
        gate: 1.0,
        degree: 0,
    };

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for _ in 0..256 {
        let value = compiled.next_with_controls(controls).left().value();
        min = min.min(value);
        max = max.max(value);
    }
    assert!(max - min > 0.1, "{min} {max}");
}

#[test]
fn gui_reverb_emits_a_wet_tail_after_an_impulse() {
    let mut state = GuiState::default();
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Right);
    place_palette_module(&mut state, ModuleCategory::Effect, "Reverb");
    state.apply(GuiAction::Edit);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::TypeValue);
    replace_prompt_text(&mut state, "1.0");
    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::TypeValue);
    replace_prompt_text(&mut state, "0.0");
    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::TypeValue);
    replace_prompt_text(&mut state, "1.0");
    state.apply(GuiAction::Confirm);
    state.apply(GuiAction::Cancel);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Output);
    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();

    compiled.next_with_controls(PatchControls {
        frequency: Hertz::new(440.0),
        gate: 1.0,
        degree: 0,
    });
    let tail = (0..12_000)
        .map(|_| {
            compiled
                .next_with_controls(PatchControls {
                    frequency: Hertz::new(440.0),
                    gate: 0.0,
                    degree: 0,
                })
                .left()
                .value()
                .abs()
        })
        .fold(0.0_f32, f32::max);

    assert!(tail > 0.02, "reverb tail peak was {tail}");
}

#[test]
fn probe_branch_does_not_need_to_feed_audio_output() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Probe);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Left);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Output);

    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    compiled.next_with_controls(PatchControls {
        frequency: Hertz::new(330.0),
        gate: 1.0,
        degree: 0,
    });
    let values = compiled.probe_values().collect::<Vec<_>>();

    assert_eq!(values.len(), 1);
    assert!((values[0].1.value() - 1.0).abs() < 0.001, "{values:?}");
}

#[test]
fn composition_probe_branch_receives_parent_input_without_feeding_output() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::CompositionInput);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Probe);
    state.apply(GuiAction::ExitComposition);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Left);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Output);

    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    compiled.next_with_controls(PatchControls {
        frequency: Hertz::new(330.0),
        gate: 1.0,
        degree: 0,
    });
    let values = compiled.probe_values().collect::<Vec<_>>();

    assert_eq!(values.len(), 1);
    assert!((values[0].1.value() - 1.0).abs() < 0.001, "{values:?}");
}

#[test]
fn overlapping_modules_are_excluded_from_audio_patch() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 7, 0);

    assert!(
        state
            .compile_audio_patch(SampleRate::new(44_100).unwrap())
            .is_ok()
    );

    state.apply(GuiAction::Move);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Confirm);

    assert_eq!(
        state
            .modules()
            .iter()
            .filter(|module| module.disabled())
            .count(),
        2
    );
    assert!(matches!(
        state.compile_audio_patch(SampleRate::new(44_100).unwrap()),
        Err(AudioPatchError::MissingOutput)
    ));
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
    assert_eq!(osc.kind(), ModuleKind::Phase);
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
fn oscillator_composition_exposes_only_frequency() {
    let mut state = GuiState::new(8, 8);
    place_palette_module(&mut state, ModuleCategory::Source, "Sine Oscillator");

    let parameters = state.modules()[0].parameters();
    assert_eq!(parameters.len(), 2);
    assert_eq!(parameters[0].name(), "Name");
    assert_eq!(parameters[1].name(), "Frequency");
}

#[test]
fn output_gain_connection_is_not_treated_as_output_signal() {
    let rate = SampleRate::new(44_100).unwrap();
    let controls_high = PatchControls {
        frequency: Hertz::new(330.0),
        gate: 1.0,
        degree: 0,
    };
    let controls_low = PatchControls {
        frequency: Hertz::new(330.0),
        gate: 0.0,
        degree: 0,
    };

    let mut state = GuiState::new(8, 8);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Up);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Output);

    let mut high = state.compile_audio_patch(rate).unwrap();
    let mut low = state.compile_audio_patch(rate).unwrap();

    let mut high_max = 0.0f32;
    let mut low_max = 0.0f32;
    for _ in 0..256 {
        let high_value = high.next_with_controls(controls_high).left().value();
        let low_value = low.next_with_controls(controls_low).left().value();
        high_max = high_max.max(high_value.abs());
        low_max = low_max.max(low_value.abs());
    }

    assert!(high_max > 0.9, "{high_max}");
    assert!(low_max < 0.0001, "{low_max}");
}

#[test]
fn unrelated_composition_does_not_silence_direct_osc_output() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::CompositionOutput);
    state.apply(GuiAction::ExitComposition);
    state.apply(GuiAction::Down);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Phase);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Output);

    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for _ in 0..256 {
        let value = compiled.next().left().value();
        min = min.min(value);
        max = max.max(value);
    }

    assert!(max - min > 0.1, "{min} {max}");
}

#[test]
fn composition_parent_inputs_route_by_port_order() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Up);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::CompositionInput);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::CompositionInput);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::CompositionOutput);
    state.apply(GuiAction::ExitComposition);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Output);

    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    let value = compiled
        .next_with_controls(PatchControls {
            frequency: Hertz::new(0.25),
            gate: 1.0,
            degree: 0,
        })
        .left()
        .value();

    assert!((value - 0.25).abs() < 0.001, "{value}");
}

#[test]
fn composition_parent_outputs_route_by_port_order() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::EditComposition);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::CompositionOutput);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::CompositionOutput);
    state.apply(GuiAction::ExitComposition);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Output);

    let mut compiled = state
        .compile_audio_patch(SampleRate::new(44_100).unwrap())
        .unwrap();
    let value = compiled
        .next_with_controls(PatchControls {
            frequency: Hertz::new(0.25),
            gate: 1.0,
            degree: 0,
        })
        .left()
        .value();

    assert!((value - 0.25).abs() < 0.001, "{value}");
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
fn selection_move_keeps_right_side_grab_over_group_at_left_edge() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Right);
    place_module(&mut state, 0, 1);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::LeftFast);

    assert_eq!(state.cursor(), GridPos::new(1, 0));

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    assert_point_near(
        pane.location(view_module_id(GridPos::new(0, 0), true))
            .unwrap(),
        pane.location(view_cell_id(GridPos::new(0, 0))).unwrap(),
    );
    assert_point_near(
        pane.location(view_module_id(GridPos::new(1, 0), true))
            .unwrap(),
        pane.location(view_cell_id(GridPos::new(1, 0))).unwrap(),
    );
}

#[test]
fn haven_single_module_copy_renders_preview_at_cursor() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Copy);
    state.apply(GuiAction::Right);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    assert!(
        pane.location(view_module_id(GridPos::new(0, 0), false))
            .is_some()
    );
    assert_point_near(
        pane.location(view_module_id(GridPos::new(0, 0), true))
            .unwrap(),
        pane.location(view_cell_id(GridPos::new(1, 0))).unwrap(),
    );
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
fn haven_grid_drag_release_in_gap_ends_selection_move() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);
    state.apply(GuiAction::Select);
    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    let start = grid_cell_center(&pane, 0, 0);
    let destination = grid_cell_center(&pane, 1, 0);
    let next_cell = grid_cell_origin(&pane, GridPos::new(2, 0));
    pane.move_to(&mut state, start);
    pane.press_button(&mut state, MouseButton::Left);
    pane.move_to(&mut state, destination);
    pane.move_to(&mut state, Point::new(next_cell.x - 1., destination.y));
    pane.release_button(&mut state, MouseButton::Left);

    assert_eq!(
        state.mode(),
        Mode::Select {
            anchor: GridPos::new(1, 0)
        }
    );
    assert!(state.module_at(GridPos::new(1, 0)).is_some());
}

#[test]
fn haven_grid_view_scrolls_after_two_cell_edge_margin() {
    let mut state = GuiState::default();
    let mut pane = PaneBuilder::new("main", main_view).build();
    let (frame, _) = pane.redraw(&mut state, 920, 720, 1.0);

    let area = grid_area(&pane, &frame);
    let columns = visible_cells(area.width);
    let rows = visible_cells(area.height);
    let right_edge = columns - 3;
    let scrolled_right = right_edge + 1;
    let bottom_edge = rows - 3;
    let scrolled_down = bottom_edge + 1;
    let origin = pane.location(view_cell_id(GridPos::new(0, 0))).unwrap();

    press_key(&mut pane, &mut state, Key::character("l"), right_edge);
    pane.redraw(&mut state, 920, 720, 1.0);
    assert_point_near(
        pane.location(view_cell_id(GridPos::new(0, 0))).unwrap(),
        origin,
    );

    press_key(&mut pane, &mut state, Key::character("l"), 1);
    pane.redraw(&mut state, 920, 720, 1.0);
    assert_point_near(
        pane.location(view_cell_id(GridPos::new(1, 0))).unwrap(),
        origin,
    );
    assert_eq!(state.cursor(), GridPos::new(scrolled_right, 0));

    press_key(&mut pane, &mut state, Key::character("j"), scrolled_down);
    pane.redraw(&mut state, 920, 720, 1.0);
    assert_point_near(
        pane.location(view_cell_id(GridPos::new(1, 1))).unwrap(),
        origin,
    );
    assert_eq!(state.cursor(), GridPos::new(scrolled_right, scrolled_down));

    press_key(
        &mut pane,
        &mut state,
        Key::character("h"),
        scrolled_right - 3,
    );
    pane.redraw(&mut state, 920, 720, 1.0);
    assert_point_near(
        pane.location(view_cell_id(GridPos::new(1, 1))).unwrap(),
        origin,
    );
    assert_eq!(state.cursor(), GridPos::new(3, scrolled_down));

    press_key(&mut pane, &mut state, Key::character("h"), 1);
    pane.redraw(&mut state, 920, 720, 1.0);
    assert_point_near(
        pane.location(view_cell_id(GridPos::new(0, 1))).unwrap(),
        origin,
    );
    assert_eq!(state.cursor(), GridPos::new(2, scrolled_down));

    press_key(
        &mut pane,
        &mut state,
        Key::character("k"),
        scrolled_down - 3,
    );
    pane.redraw(&mut state, 920, 720, 1.0);
    assert_point_near(
        pane.location(view_cell_id(GridPos::new(0, 1))).unwrap(),
        origin,
    );
    assert_eq!(state.cursor(), GridPos::new(2, 3));

    press_key(&mut pane, &mut state, Key::character("k"), 1);
    pane.redraw(&mut state, 920, 720, 1.0);
    assert_point_near(
        pane.location(view_cell_id(GridPos::new(0, 0))).unwrap(),
        origin,
    );
    assert_eq!(state.cursor(), GridPos::new(2, 2));
}

#[test]
fn haven_selection_move_preview_offsets_clipped_grid() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    assert!(pane.location(10_001).is_none());
    assert!(pane.location(10_011).is_none());
    assert_point_near(
        pane.location(110_001).unwrap(),
        pane.location(10_110).unwrap(),
    );
    assert_point_near(
        pane.location(110_011).unwrap(),
        pane.location(10_120).unwrap(),
    );
}

#[test]
fn haven_single_module_move_preview_ignores_overlapped_modules() {
    let mut state = GuiState::default();
    place_palette_module(&mut state, ModuleCategory::Shape, "ADSR");
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Down);
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Confirm);

    state.apply(GuiAction::Move);
    state.apply(GuiAction::Right);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    assert!(
        pane.location(view_module_id(GridPos::new(0, 1), false))
            .is_some()
    );
    assert!(
        pane.location(view_module_id(GridPos::new(0, 1), true))
            .is_none()
    );
    assert_point_near_x(
        pane.location(view_module_id(GridPos::new(0, 0), true))
            .unwrap(),
        pane.location(view_cell_id(GridPos::new(1, 0))).unwrap().x,
    );
}

#[test]
fn haven_module_move_preview_renders_inside_composition() {
    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Composition);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::EditComposition);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 640, 480, 1.0);

    assert_point_near(
        pane.location(view_module_id(GridPos::new(1, 0), true))
            .unwrap(),
        pane.location(view_cell_id(GridPos::new(0, 0))).unwrap(),
    );
}

#[test]
fn haven_selection_move_preview_shifts_viewport_at_grid_edge() {
    let mut state = GuiState::default();
    for _ in 0..14 {
        state.apply(GuiAction::Right);
    }
    place_module_kind(&mut state, ModuleKind::Freq);
    state.apply(GuiAction::Right);
    place_module_kind(&mut state, ModuleKind::Gate);
    state.apply(GuiAction::Left);
    state.apply(GuiAction::Select);
    state.apply(GuiAction::Right);
    state.apply(GuiAction::Move);
    state.apply(GuiAction::RightFast);

    let mut pane = PaneBuilder::new("main", main_view).build();
    pane.redraw(&mut state, 920, 720, 1.0);

    assert!(
        pane.location(view_module_id(GridPos::new(14, 0), false))
            .is_none()
    );
    assert!(
        pane.location(view_module_id(GridPos::new(15, 0), false))
            .is_none()
    );
    assert_point_near(
        pane.location(view_module_id(GridPos::new(14, 0), true))
            .unwrap(),
        pane.location(view_cell_id(GridPos::new(18, 0))).unwrap(),
    );
    assert_point_near(
        pane.location(view_module_id(GridPos::new(15, 0), true))
            .unwrap(),
        pane.location(view_cell_id(GridPos::new(19, 0))).unwrap(),
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
    assert_eq!(state.instrument_count(), 5);

    place_module(&mut state, 0, 0);

    state.apply(GuiAction::Instrument(1));
    assert_eq!(state.active_instrument(), 1);
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

    state.apply(GuiAction::Instrument(4));
    assert_eq!(state.active_instrument(), 4);
    assert!(state.modules().is_empty());
}

#[test]
fn global_toggles_match_normal_mode_actions() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::TogglePlay);
    state.apply(GuiAction::ToggleMeters);

    assert!(state.playing());
    assert!(state.show_meters());
}

#[test]
fn load_action_sets_one_shot_load_request() {
    let mut state = GuiState::new(8, 8);

    state.apply(GuiAction::Load);

    assert!(state.take_load_request());
    assert!(!state.take_load_request());
}

#[test]
fn loading_dirty_project_requires_an_explicit_choice() {
    let mut state = GuiState::new(8, 8);
    place_module(&mut state, 0, 0);

    state.apply(GuiAction::Load);
    assert_eq!(state.mode(), Mode::LoadConfirm);
    assert!(!state.take_load_request());

    state.apply(GuiAction::Cancel);
    assert_eq!(state.mode(), Mode::Normal);
    assert!(!state.take_load_request());

    state.apply(GuiAction::Load);
    state.apply(GuiAction::Delete);
    assert_eq!(state.mode(), Mode::Normal);
    assert!(state.take_load_request());
}

#[test]
fn loading_restores_sample_path_and_relink_clears_missing_state() {
    let path = project_path("sample-load");
    let sample_name = "missing-sample.wav";
    let project = brainwash_grid::project::Project {
        bpm: 120.0,
        bars: 1.0,
        scale_idx: 0,
        modules: vec![brainwash_grid::project::ModuleDef {
            id: brainwash_grid::ModuleId::new(1),
            kind: brainwash_grid::project::ModuleKind::Standard(
                brainwash_grid::project::StandardModule::Sample,
            ),
            x: 0,
            y: 0,
            orientation: brainwash_grid::Orientation::Right,
            params: brainwash_grid::project::ModuleParams::Sample {
                file_idx: 0,
                file_name: sample_name.to_string(),
                samples: std::sync::Arc::new(Vec::new()),
                position: 0.0,
                connected: 255,
            },
        }],
        track: None,
        compositions: Vec::new(),
    };
    brainwash_grid::project::save(&path, &project).unwrap();

    let mut state = GuiState::new(8, 8);
    state.load_project(&path).unwrap();
    assert_eq!(
        state.modules()[0].parameters()[0].value_label(),
        format!("missing: {sample_name}")
    );
    assert!(state.document_status().contains("missing sample files"));

    let relinked = project_path("relinked-sample");
    let mut writer = hound::WavWriter::create(
        &relinked,
        hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    writer.write_sample(0_i16).unwrap();
    writer.finalize().unwrap();
    let module = state.modules()[0].id();
    assert!(state.relink_sample(module, &relinked));
    assert_eq!(
        state.modules()[0].parameters()[0].value_label(),
        relinked.to_string_lossy()
    );
    assert!(state.dirty());

    let saved = project_path("sample-relinked-save");
    assert!(state.save_project(&saved));
    let saved_project = brainwash_grid::project::load(&saved).unwrap();
    let brainwash_grid::project::ModuleParams::Sample { file_name, .. } =
        &saved_project.modules[0].params
    else {
        panic!("expected sample module parameters");
    };
    assert_eq!(file_name, &relinked.to_string_lossy());

    let _ = fs::remove_file(path);
    let _ = fs::remove_file(relinked);
    let _ = fs::remove_file(saved);
}

#[test]
fn load_project_replaces_state_from_project_file() {
    let path = project_path("load");
    fs::write(
        &path,
        r#"(
  bpm: 132.0,
  bars: 1.0,
  scale_idx: 2,
  modules: [
    (
      id: 7,
      kind: Standard(Phase),
      x: 1,
      y: 2,
      params: Phase(
        frequency: 440.0,
        connected: 255,
      ),
    ),
    (
      id: 8,
      kind: Standard(Output),
      x: 3,
      y: 2,
      params: Output(
        input: 0.0,
        gain: 0.5,
        connected: 255,
      ),
    ),
  ],
  track: Some("(0/4)"),
  compositions: [],
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
        .find(|module| module.kind() == ModuleKind::Phase)
        .unwrap();
    assert_eq!(osc.position(), GridPos::new(1, 2));
    assert_eq!(osc.label(), "Phase");
    assert_eq!(osc.parameters().len(), 1);

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

#[test]
fn load_project_rejects_module_kind_and_params_disagreement() {
    let path = project_path("mismatched-params");
    fs::write(
        &path,
        r#"(
  bpm: 120.0,
  bars: 1.0,
  scale_idx: 0,
  modules: [
    (
      id: 1,
      kind: Standard(Output),
      x: 0,
      y: 0,
      params: Osc(
        wave: Sin,
        freq: (unit: Hz, seconds: 999.0, samples: 999.0, bar_num: 1, bar_denom: 4, hz: 440.0),
        shift: 0.0,
        gain: 1.0,
        uni: false,
        connected: 255,
      ),
    ),
  ],
  track: None,
  compositions: [],
)"#,
    )
    .unwrap();

    let mut state = GuiState::new(8, 8);
    place_module_kind(&mut state, ModuleKind::Gate);
    let error = state.load_project(&path).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(state.modules().len(), 1);
    assert_eq!(state.modules()[0].kind(), ModuleKind::Gate);

    let _ = fs::remove_file(path);
}

#[test]
fn load_project_rejects_modules_outside_the_fixed_grid() {
    let path = project_path("out-of-bounds");
    let project = brainwash_grid::project::Project {
        bpm: 120.0,
        bars: 1.0,
        scale_idx: 0,
        modules: vec![brainwash_grid::project::ModuleDef {
            id: brainwash_grid::ModuleId::new(1),
            kind: brainwash_grid::project::ModuleKind::Standard(
                brainwash_grid::project::StandardModule::Gate,
            ),
            x: 8,
            y: 0,
            orientation: brainwash_grid::Orientation::Right,
            params: brainwash_grid::project::ModuleParams::None,
        }],
        track: None,
        compositions: Vec::new(),
    };
    brainwash_grid::project::save(&path, &project).unwrap();

    let mut state = GuiState::new(8, 8);
    let error = state.load_project(&path).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(state.modules().is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn load_project_rejects_invalid_active_time_value() {
    let path = project_path("invalid-time");
    fs::write(
        &path,
        r#"(
  bpm: 120.0,
  bars: 1.0,
  scale_idx: 0,
  modules: [
    (
      id: 1,
      kind: Standard(Rise),
      x: 0,
      y: 0,
      params: Rise(
        time: (unit: Bars, seconds: 0.1, samples: 100.0, bar_num: 1, bar_denom: 0, hz: 12.0),
        connected: 255,
      ),
    ),
  ],
  track: None,
  compositions: [],
)"#,
    )
    .unwrap();

    let mut state = GuiState::new(8, 8);
    let error = state.load_project(&path).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(state.modules().is_empty());

    let _ = fs::remove_file(path);
}

#[test]
fn load_project_rejects_invalid_inactive_time_value() {
    let path = project_path("inactive-time");
    fs::write(
        &path,
        r#"(
  bpm: 120.0,
  bars: 1.0,
  scale_idx: 0,
  modules: [
    (
      id: 1,
      kind: Standard(Rise),
      x: 0,
      y: 0,
      params: Rise(
        time: (unit: Hz, seconds: 999.0, samples: 999.0, bar_num: 1, bar_denom: 0, hz: 12.0),
        connected: 255,
      ),
    ),
  ],
  track: None,
  compositions: [],
)"#,
    )
    .unwrap();

    let mut state = GuiState::new(8, 8);
    let error = state.load_project(&path).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(state.modules().is_empty());

    let _ = fs::remove_file(path);
}

fn project_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "brainwash-gui-{name}-{}-{}.bw",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    path
}

fn replace_prompt_text(state: &mut GuiState, text: &str) {
    state.apply(GuiAction::TextStart);
    while !state.prompt_text().is_empty() {
        state.apply(GuiAction::DeleteChar);
    }
    for character in text.chars() {
        state.apply(GuiAction::InputChar(character));
    }
}

fn replace_prompt_text_pane(pane: &mut Pane<GuiState>, state: &mut GuiState, text: &str) {
    pane.key_pressed(state, NamedKey::Home);
    while !state.prompt_text().is_empty() {
        pane.key_pressed(state, NamedKey::Delete);
    }
    for character in text.chars() {
        pane.key_pressed(state, Key::character(character.to_string()));
    }
}

fn place_module(state: &mut GuiState, category_rights: usize, selection_downs: usize) {
    let kind = match (category_rights, selection_downs) {
        (0, 0) => ModuleKind::Freq,
        (0, 1) => ModuleKind::Gate,
        (0, 2) => ModuleKind::Degree,
        (0, 4) => ModuleKind::Phase,
        (0, 5) => ModuleKind::Random,
        (0, 6) => ModuleKind::Sample,
        (1, 0) => ModuleKind::Rise,
        (1, 1) => ModuleKind::Fall,
        (1, 2) => ModuleKind::Ramp,
        (1, 4) => ModuleKind::Envelope,
        (2, 0) => ModuleKind::Filter,
        (2, 1) => ModuleKind::Damp,
        (3, 0) => ModuleKind::Comb,
        (3, 1) => ModuleKind::Allpass,
        (3, 2) => ModuleKind::Delay,
        (3, 3) => ModuleKind::DelayTap,
        (4, 0) => ModuleKind::Probe,
        (4, 1) => ModuleKind::Multiply,
        (4, 2) => ModuleKind::Add,
        (4, 3) => ModuleKind::GreaterThan,
        (4, 4) => ModuleKind::LessThan,
        (4, 5) => ModuleKind::Switch,
        (5, 0) => ModuleKind::TurnRightDown,
        (5, 1) => ModuleKind::TurnDownRight,
        (5, 2) => ModuleKind::LeftSplit,
        (5, 3) => ModuleKind::TopSplit,
        (5, 4) => ModuleKind::RightJoin,
        (5, 5) => ModuleKind::DownJoin,
        (6, 0) => ModuleKind::CompositionInput,
        (6, 1) => ModuleKind::CompositionOutput,
        (6, 2) => ModuleKind::Composition,
        (7, 0) => ModuleKind::Output,
        _ => panic!("unknown module fixture"),
    };
    place_module_kind(state, kind);
}

fn place_module_kind(state: &mut GuiState, kind: ModuleKind) {
    state.apply(GuiAction::Palette(kind.category()));
    for _ in all_modules() {
        state.apply(GuiAction::PaletteUp);
    }
    let index = all_modules()
        .iter()
        .filter(|module| module.category() == kind.category())
        .position(|module| *module == kind)
        .unwrap();
    for _ in 0..index {
        state.apply(GuiAction::PaletteDown);
    }
    state.apply(GuiAction::Confirm);
}

fn place_palette_module(state: &mut GuiState, category: ModuleCategory, label: &str) {
    state.apply(GuiAction::Palette(category));
    for _ in all_modules() {
        state.apply(GuiAction::PaletteUp);
    }
    while state.selected_palette_label() != label {
        state.apply(GuiAction::PaletteDown);
    }
    state.apply(GuiAction::Confirm);
}

fn grid_cell_center(pane: &haven::Pane<GuiState>, x: u16, y: u16) -> Point {
    const CELL: f32 = 30.;
    let origin = grid_cell_origin(pane, GridPos::new(x, y));
    Point::new(origin.x + CELL * 0.5, origin.y + CELL * 0.5)
}

fn grid_cell_origin(pane: &haven::Pane<GuiState>, position: GridPos) -> Point {
    const CELL: f32 = 30.;
    let center = pane.location(view_cell_id(position)).unwrap();
    Point::new(center.x - CELL * 0.5, center.y - CELL * 0.5)
}

fn grid_area(pane: &haven::Pane<GuiState>, frame: &Frame) -> haven::Area {
    let center = pane.location(990).unwrap();
    frame
        .items
        .iter()
        .filter_map(|item| match item {
            RenderItem::Path { area, .. }
                if (area.x + area.width * 0.5 - center.x).abs() < 0.5
                    && (area.y + area.height * 0.5 - center.y).abs() < 0.5
                    && area.width > 30.
                    && area.height > 30. =>
            {
                Some(*area)
            }
            _ => None,
        })
        .min_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
        .unwrap()
}

fn visible_cells(length: f32) -> u16 {
    const CELL: f32 = 30.;
    const GAP: f32 = 3.;
    ((length + GAP) / (CELL + GAP)).floor() as u16
}

fn press_key(pane: &mut haven::Pane<GuiState>, state: &mut GuiState, key: Key, count: u16) {
    for _ in 0..count {
        pane.key_pressed(state, key.clone());
    }
}

fn view_cell_id(position: GridPos) -> u64 {
    if position.x < 10 && position.y < 10 {
        10_000 + position.y as u64 * 100 + position.x as u64 * 10
    } else {
        700_000 + position.y as u64 * 1_000 + position.x as u64 * 10
    }
}

fn view_module_id(position: GridPos, preview: bool) -> u64 {
    view_cell_id(position) + 1 + if preview { 100_000 } else { 0 }
}

fn assert_point_near(actual: Point, expected: Point) {
    let dx = (actual.x - expected.x).abs();
    let dy = (actual.y - expected.y).abs();
    assert!(dx < 0.5 && dy < 0.5, "{actual:?} != {expected:?}");
}

fn assert_point_near_x(actual: Point, expected: f32) {
    assert!(
        (actual.x - expected).abs() < 0.5,
        "{actual:?} != {expected}"
    );
}

fn assert_point_near_y(actual: Point, expected: f32) {
    assert!(
        (actual.y - expected).abs() < 0.5,
        "{actual:?} != {expected}"
    );
}
