use crate::model::{
    GridPointerPhase, GridPos, GuiAction, GuiState, Mode, ModuleCategory, ModuleKind, Orientation,
    ParameterValue,
};
use haven::*;

const ROOT: u64 = 1;
const GRID_COLUMNS: u16 = 16;
const GRID_ROWS: u16 = 12;
const CELL: f32 = 30.;
const GAP: f32 = 3.;
const TILE_PAD: f32 = 3.;
const PORT_SIZE: f32 = 7.;
const PORT_INSET: f32 = 1.;
const WIRE_THICKNESS: f32 = 4.;
const PALETTE_ROW_HEIGHT: f32 = 23.;

pub fn main_view<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let mut layers = vec![
        key_surface(app).expand(),
        column_spaced(
            10.,
            vec![
                toolbar(state, app).height(34.),
                patch_panel(state, app).expand(),
            ],
        )
        .pad(10.)
        .expand(),
    ];
    if let Some(panel) = context_panel(state, app) {
        layers.push(panel.pad_x(10.).pad_y(54.).align(Align::TopTrailing));
    }
    if matches!(
        state.mode(),
        Mode::QuitConfirm
            | Mode::ValueInput { .. }
            | Mode::SavePrompt
            | Mode::SaveConfirm
            | Mode::ExportPrompt
            | Mode::ExportConfirm
            | Mode::TrackPrompt
            | Mode::TrackSettings { .. }
    ) {
        layers.push(
            prompt_panel(state, app)
                .width(360.)
                .height(178.)
                .align(Align::CenterCenter),
        );
    }
    stack(layers).align(Align::TopLeading)
}

fn key_surface(app: &mut PaneState) -> View<'static, GuiState> {
    stack(vec![
        rect(ROOT)
            .fill(bg())
            .view()
            .gesture(key("h", GuiAction::Left))
            .gesture(key("j", GuiAction::Down))
            .gesture(key("k", GuiAction::Up))
            .gesture(key("l", GuiAction::Right))
            .gesture(key("H", GuiAction::LeftFast))
            .gesture(key("J", GuiAction::DownFast))
            .gesture(key("K", GuiAction::UpFast))
            .gesture(key("L", GuiAction::RightFast))
            .gesture(new_or_no_key())
            .gesture(key("m", GuiAction::Move))
            .gesture(yes_key("y", GuiAction::Copy))
            .gesture(key("i", GuiAction::Edit))
            .gesture(key("o", GuiAction::Rotate))
            .gesture(env_key(".", GuiAction::Delete, GuiAction::DeletePoint))
            .gesture(key("c", GuiAction::ToggleCurve))
            .gesture(key("r", GuiAction::Delete))
            .gesture(key(",", GuiAction::Select))
            .gesture(key("p", GuiAction::EditSubpatch))
            .gesture(key_named(NamedKey::Space, GuiAction::TogglePlay))
            .gesture(key("v", GuiAction::ToggleMeters))
            .gesture(key("O", GuiAction::Load))
            .gesture(key("w", GuiAction::Save))
            .gesture(key("W", GuiAction::SaveAs))
            .gesture(key("e", GuiAction::Export))
            .gesture(key("Q", GuiAction::Quit))
            .gesture(yes_key("Y", GuiAction::Confirm))
            .gesture(key("N", GuiAction::Cancel))
            .gesture(key("/", GuiAction::Search))
            .gesture(edit_key("u", GuiAction::Undo, GuiAction::CycleUnit))
            .gesture(key("U", GuiAction::Redo))
            .gesture(key(";", GuiAction::TogglePort))
            .gesture(step_key())
            .gesture(edit_key("t", GuiAction::TrackEdit, GuiAction::TypeValue))
            .gesture(key("+", GuiAction::NewInstrument))
            .gesture(key("[", GuiAction::HelpScrollUp))
            .gesture(key("]", GuiAction::HelpScrollDown))
            .gesture(key("1", GuiAction::Instrument(0)))
            .gesture(key("2", GuiAction::Instrument(1)))
            .gesture(key("3", GuiAction::Instrument(2)))
            .gesture(key("4", GuiAction::Instrument(3)))
            .gesture(key("5", GuiAction::Instrument(4)))
            .gesture(key_named(NamedKey::Enter, GuiAction::Confirm))
            .gesture(key_named(NamedKey::Escape, GuiAction::Cancel))
            .gesture(key_named(NamedKey::Backspace, GuiAction::Backspace))
            .gesture(key_named(NamedKey::Delete, GuiAction::DeleteChar))
            .gesture(key_named(NamedKey::Home, GuiAction::TextStart))
            .gesture(key_named(NamedKey::End, GuiAction::TextEnd))
            .gesture(key_named(NamedKey::ArrowLeft, GuiAction::Left))
            .gesture(key_named(NamedKey::ArrowDown, GuiAction::Down))
            .gesture(key_named(NamedKey::ArrowUp, GuiAction::Up))
            .gesture(key_named(NamedKey::ArrowRight, GuiAction::Right))
            .build(app)
            .expand(),
        text_input_keys(app).expand(),
    ])
}

fn toolbar<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    row_spaced(
        6.,
        vec![
            status_chip(status_text(state), accent(), app),
            action_button(
                if state.playing() { "Pause" } else { "Play" },
                GuiAction::TogglePlay,
                state.playing(),
                app,
            ),
            action_button("Meters", GuiAction::ToggleMeters, state.show_meters(), app),
            action_button("Load", GuiAction::Load, false, app),
            action_button("Save", GuiAction::Save, false, app),
            action_button("Export", GuiAction::Export, false, app),
            action_button("Track", GuiAction::TrackEdit, false, app),
            action_button("Inst", GuiAction::NewInstrument, false, app),
            status_chip(
                format!(
                    "{} / {}",
                    state.active_instrument() + 1,
                    state.instrument_count()
                ),
                panel(),
                app,
            ),
        ],
    )
}

fn context_panel<'a>(state: &'a GuiState, app: &mut PaneState) -> Option<View<'a, GuiState>> {
    match state.mode() {
        Mode::Palette => Some(palette_panel(state, app)),
        Mode::Edit { .. }
        | Mode::ValueInput { .. }
        | Mode::AdsrEdit { .. }
        | Mode::EnvEdit { .. }
        | Mode::ProbeEdit { .. }
        | Mode::SampleView { .. } => Some(edit_panel(state, app)),
        _ if state.palette_searching() => Some(palette_panel(state, app)),
        _ => None,
    }
}

fn palette_panel<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let (width, height) = palette_panel_size(state);
    let content = if state.palette_searching() {
        let filter = state.palette_filter().to_string();
        let modules = state
            .filtered_palette_modules()
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                filtered_module_choice(
                    index,
                    *kind,
                    Some(*kind) == state.selected_filtered_palette_module(),
                    app,
                )
            })
            .collect::<Vec<_>>();
        vec![
            stack(vec![
                rect(210)
                    .fill(field())
                    .stroke(accent(), Stroke::new(1.))
                    .corner_rounding(6.)
                    .build(app)
                    .inert(),
                text(211, format!("/{filter}"))
                    .font_size(14)
                    .fill(fg())
                    .view()
                    .build(app)
                    .pad_x(9.)
                    .pad_y(7.),
            ]),
            column_spaced(6., modules),
        ]
    } else {
        let categories = ModuleCategory::ALL
            .iter()
            .map(|category| category_button(*category, *category == state.palette_category(), app))
            .collect::<Vec<_>>();
        let modules = state
            .palette_modules()
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                let selected =
                    state.mode() == Mode::Palette && *kind == state.selected_palette_module();
                module_choice(index, *kind, selected, app)
            })
            .collect::<Vec<_>>();
        vec![
            row_spaced(6., categories),
            column_spaced(6., modules),
        ]
    };

    stack(vec![
        rect(200)
            .fill(panel())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(8.)
            .build(app)
            .width(width)
            .height(height)
            .inert(),
        column_spaced(
            12.,
            vec![
                text(201, "Modules")
                    .font_size(18)
                    .fill(fg())
                    .view()
                    .build(app),
                column_spaced(6., content),
            ],
        )
        .pad(12.),
    ])
    .width(width)
    .height(height)
}

fn palette_panel_size(state: &GuiState) -> (f32, f32) {
    const PANEL_PAD: f32 = 24.;
    const TITLE_H: f32 = 22.;
    const TITLE_GAP: f32 = 12.;
    const SEARCH_H: f32 = 28.;
    const GAP: f32 = 6.;

    let title_w = text_width("Modules", 18.) + PANEL_PAD;
    if state.palette_searching() {
        let modules = state.filtered_palette_modules();
        let module_w = modules
            .iter()
            .map(|kind| row_width(kind.label(), 13.))
            .fold(0., f32::max);
        let search_w = text_width(&format!("/{}", state.palette_filter()), 14.) + 18.;
        let width = title_w.max(module_w + PANEL_PAD).max(search_w + PANEL_PAD);
        let height = PANEL_PAD
            + TITLE_H
            + TITLE_GAP
            + SEARCH_H
            + GAP
            + rows_height(modules.len(), PALETTE_ROW_HEIGHT, GAP);
        (width, height)
    } else {
        let modules = state.palette_modules();
        let module_w = modules
            .iter()
            .map(|kind| row_width(kind.label(), 13.))
            .fold(0., f32::max);
        let category_w = ModuleCategory::ALL
            .iter()
            .map(|category| row_width(category.label(), 13.))
            .sum::<f32>()
            + GAP * ModuleCategory::ALL.len().saturating_sub(1) as f32;
        let width = title_w.max(module_w + PANEL_PAD).max(category_w + PANEL_PAD);
        let height = PANEL_PAD
            + TITLE_H
            + TITLE_GAP
            + PALETTE_ROW_HEIGHT
            + GAP
            + rows_height(modules.len(), PALETTE_ROW_HEIGHT, GAP);
        (width, height)
    }
}

fn rows_height(count: usize, row_height: f32, gap: f32) -> f32 {
    if count == 0 {
        0.
    } else {
        count as f32 * row_height + count.saturating_sub(1) as f32 * gap
    }
}

fn row_width(label: &str, font_size: f32) -> f32 {
    text_width(label, font_size) + 20.
}

fn text_width(label: &str, font_size: f32) -> f32 {
    label.chars().count() as f32 * font_size * 0.62
}

fn patch_panel<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    stack(vec![
        rect(300)
            .fill(Color::from_rgb8(18, 20, 24))
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(8.)
            .build(app),
        column_spaced(
            10.,
            vec![
                row_spaced(
                    8.,
                    vec![
                        text(301, "Patch")
                            .font_size(18)
                            .fill(fg())
                            .view()
                            .build(app),
                        status_chip(mode_text(state), panel(), app),
                        status_chip(format!("Sub {}", state.subpatch_depth()), panel(), app),
                    ],
                )
                .height(28.),
                patch_grid(state, app),
                shortcut_bar(state, app).height(34.),
            ],
        )
        .pad(10.),
    ])
}

fn patch_grid<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let rows = (0..GRID_ROWS)
        .map(|y| {
            let cells = (0..GRID_COLUMNS)
                .map(|x| grid_cell(state, GridPos::new(x, y), app))
                .collect::<Vec<_>>();
            row_spaced(GAP, cells).height(CELL)
        })
        .collect::<Vec<_>>();
    stack_aligned(
        Align::TopLeading,
        vec![
            column_spaced(GAP, rows),
            connection_layer(state, app),
            module_layer(state, app),
            grid_pointer_surface(app)
                .width(grid_width())
                .height(grid_height()),
        ],
    )
    .width(grid_width())
    .height(grid_height())
}

fn connection_layer<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let mut layers = Vec::new();
    for (index, connection) in state.connections().into_iter().enumerate() {
        let Some(_) = state
            .modules()
            .iter()
            .find(|module| module.id() == connection.from())
        else {
            continue;
        };
        let Some(_) = state
            .modules()
            .iter()
            .find(|module| module.id() == connection.to())
        else {
            continue;
        };
        let id = 60_000 + index as u64 * 10;
        let segment = connection_segment(
            connection.from_cell(),
            connection.orientation(),
            connection.to_cell(),
        );
        layers.push(
            rect(id)
                .fill(accent().with_alpha(0.62))
                .corner_rounding(2.)
                .build(app)
                .width(segment.width)
                .height(segment.height)
                .offset(segment.x, segment.y),
        );
    }
    stack_aligned(Align::TopLeading, layers)
        .width(grid_width())
        .height(grid_height())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WireSegment {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn connection_segment(from: GridPos, orientation: Orientation, to: GridPos) -> WireSegment {
    let (x1, y1) = port_center(from, orientation, true);
    let (x2, y2) = port_center(to, orientation, false);
    if matches!(orientation, Orientation::Right) {
        WireSegment {
            x: x1.min(x2),
            y: y1 - WIRE_THICKNESS * 0.5,
            width: (x2 - x1).abs().max(WIRE_THICKNESS),
            height: WIRE_THICKNESS,
        }
    } else {
        WireSegment {
            x: x1 - WIRE_THICKNESS * 0.5,
            y: y1.min(y2),
            width: WIRE_THICKNESS,
            height: (y2 - y1).abs().max(WIRE_THICKNESS),
        }
    }
}

fn port_center(position: GridPos, orientation: Orientation, output: bool) -> (f32, f32) {
    let x = position.x as f32 * (CELL + GAP);
    let y = position.y as f32 * (CELL + GAP);
    let tile = CELL - TILE_PAD * 2.;
    let port = PORT_INSET + PORT_SIZE * 0.5;
    match (orientation, output) {
        (Orientation::Right, false) => (x + TILE_PAD + port, y + CELL * 0.5),
        (Orientation::Right, true) => (x + TILE_PAD + tile - port, y + CELL * 0.5),
        (Orientation::Down, false) => (x + CELL * 0.5, y + TILE_PAD + port),
        (Orientation::Down, true) => (x + CELL * 0.5, y + TILE_PAD + tile - port),
    }
}

fn module_layer<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    stack_aligned(
        Align::TopLeading,
        state
            .modules()
            .iter()
            .map(|module| {
                let position = module.position();
                module_tile(cell_id(position), module, app).offset(
                    position.x as f32 * (CELL + GAP) + TILE_PAD,
                    position.y as f32 * (CELL + GAP) + TILE_PAD,
                )
            })
            .collect(),
    )
    .width(grid_width())
    .height(grid_height())
}

fn shortcut_bar<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    row_spaced(
        6.,
        shortcuts(state)
            .iter()
            .enumerate()
            .map(|(index, (key, label))| shortcut_chip(index, key, label, app))
            .collect(),
    )
}

fn edit_panel<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let module_id = match state.mode() {
        Mode::Edit { module, .. }
        | Mode::ValueInput { module, .. }
        | Mode::AdsrEdit { module, .. }
        | Mode::EnvEdit { module, .. }
        | Mode::ProbeEdit { module }
        | Mode::SampleView { module, .. } => Some(module),
        _ => None,
    };
    let module = module_id.and_then(|id| state.modules().iter().find(|module| module.id() == id));
    let mut rows = Vec::new();
    match (state.mode(), module) {
        (Mode::Edit { parameter, .. }, Some(module)) => {
            rows.push(editor_header(500, module.kind().label(), "Parameters", app));
            for (index, parameter_row_value) in module.parameters().iter().enumerate() {
                rows.push(parameter_row(
                    index,
                    parameter == index,
                    parameter_row_value,
                    app,
                ));
            }
            if module.kind().has_visual_editor() {
                rows.push(special_editor_button(
                    parameter == module.parameters().len(),
                    app,
                ));
            }
        }
        (Mode::AdsrEdit { parameter, .. }, Some(module)) => {
            rows.push(editor_header(
                500,
                module.kind().label(),
                "Envelope shape",
                app,
            ));
            rows.push(adsr_editor(module, parameter, app));
        }
        (Mode::EnvEdit { point, editing, .. }, Some(module)) => {
            rows.push(editor_header(
                500,
                module.kind().label(),
                "Envelope points",
                app,
            ));
            rows.push(envelope_editor(module.env_points(), point, editing, app));
        }
        (Mode::ProbeEdit { .. }, Some(module)) => {
            rows.push(editor_header(
                500,
                module.kind().label(),
                "Signal probe",
                app,
            ));
            rows.push(probe_editor(state.probe_history(module.id()), state.probe_len(), app));
        }
        (Mode::SampleView { zoom, offset, .. }, Some(module)) => {
            rows.push(editor_header(
                500,
                module.kind().label(),
                "Sample window",
                app,
            ));
            rows.push(sample_editor(zoom, offset, app));
        }
        _ => {
            rows.push(
                text(500, "Select a module")
                    .font_size(14)
                    .fill(quiet())
                    .view()
                    .build(app),
            );
        }
    }
    stack(vec![
        rect(501)
            .fill(panel())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(8.)
            .build(app)
            .inert(),
        column_spaced(5., rows).pad(10.),
    ])
}

fn editor_header<'a>(
    id: u64,
    module: &'static str,
    label: &'static str,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    row_spaced(
        8.,
        vec![
            text(id, module).font_size(16).fill(fg()).view().build(app),
            text(id + 1, label)
                .font_size(13)
                .fill(quiet())
                .view()
                .build(app),
        ],
    )
}

fn parameter_row<'a>(
    index: usize,
    selected: bool,
    parameter: &'a crate::model::ModuleParameter,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = 600 + index as u64 * 10;
    stack(vec![
        rect(id)
            .fill(if selected {
                Color::from_rgb8(49, 70, 86)
            } else {
                field()
            })
            .stroke(if selected { accent() } else { line() }, Stroke::new(1.))
            .corner_rounding(6.)
            .build(app)
            .inert(),
        column_spaced(
            3.,
            vec![
                row_spaced(
                    8.,
                    vec![
                        text(id + 1, parameter.name())
                            .font_size(12)
                            .fill(fg())
                            .view()
                            .build(app),
                        text(
                            id + 3,
                            if parameter.connected() {
                                "port"
                            } else {
                                "fixed"
                            },
                        )
                        .font_size(12)
                        .fill(if parameter.connected() {
                            accent()
                        } else {
                            quiet()
                        })
                        .view()
                        .build(app),
                    ],
                ),
                text(id + 2, parameter.value_label())
                    .font_size(12)
                    .fill(quiet())
                    .view()
                    .build(app),
            ],
        )
        .pad_x(9.)
        .pad_y(6.),
    ])
}

fn settings_text(state: &GuiState, parameter: usize) -> String {
    let marker = |index| if parameter == index { ">" } else { " " };
    format!(
        "{} BPM {}   {} Scale {}   {} Voice {}",
        marker(0),
        state.bpm(),
        marker(1),
        state.scale_name(),
        marker(2),
        state.probe_voice() + 1
    )
}

fn special_editor_button<'a>(selected: bool, app: &mut PaneState) -> View<'a, GuiState> {
    let id = 780;
    stack(vec![
        rect(id)
            .fill(if selected {
                Color::from_rgb8(49, 70, 86)
            } else {
                field()
            })
            .stroke(if selected { accent() } else { line() }, Stroke::new(1.))
            .corner_rounding(6.)
            .build(app)
            .inert(),
        text(id + 1, "Open visual editor")
            .font_size(12)
            .fill(fg())
            .view()
            .build(app)
            .pad_x(9.)
            .pad_y(5.),
    ])
}

fn prompt_panel<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let (title, body) = match state.mode() {
        Mode::QuitConfirm => ("Quit", "Discard unsaved patch changes?"),
        Mode::ValueInput { .. } => ("Value", "Type a numeric value"),
        Mode::SavePrompt => ("Save", "Patch file"),
        Mode::SaveConfirm => ("Overwrite", "Replace existing patch?"),
        Mode::ExportPrompt => ("Export", "WAV file"),
        Mode::ExportConfirm => ("Overwrite", "Replace existing WAV?"),
        Mode::TrackPrompt => ("Track", "Note pattern"),
        Mode::TrackSettings { .. } => ("Settings", "Track playback"),
        _ => ("", ""),
    };
    let input = if matches!(state.mode(), Mode::TrackPrompt) {
        stack(vec![
            rect(793)
                .fill(field())
                .stroke(line(), Stroke::new(1.))
                .corner_rounding(6.)
                .build(app)
                .inert(),
            text_field(794, binding!(state.prompt_input))
                .singleline()
                .align(Alignment::Start)
                .font_size(14)
                .padding(10.)
                .on_edit(|state, _, edit| {
                    if let EditInteraction::Update(text) = edit {
                        state.sync_track_prompt_text(text);
                    }
                })
                .build(app),
        ])
    } else {
        let value = match state.mode() {
            Mode::QuitConfirm => "Press Enter to quit".to_string(),
            Mode::SaveConfirm | Mode::ExportConfirm => "Press Enter to overwrite".to_string(),
            Mode::TrackSettings { parameter } => settings_text(state, parameter),
            _ => {
                let mut text = state.prompt_text().to_string();
                text.insert(state.prompt_cursor(), '|');
                text
            }
        };
        stack(vec![
            rect(793)
                .fill(field())
                .stroke(line(), Stroke::new(1.))
                .corner_rounding(6.)
                .build(app)
                .inert(),
            text(794, value)
                .font_size(14)
                .fill(fg())
                .view()
                .build(app)
                .pad_x(10.)
                .pad_y(8.),
        ])
    };
    stack(vec![
        rect(790)
            .fill(panel())
            .stroke(accent(), Stroke::new(1.))
            .corner_rounding(8.)
            .build(app)
            .inert(),
        column_spaced(
            12.,
            vec![
                text(791, title)
                    .font_size(20)
                    .fill(fg())
                    .view()
                    .build(app)
                    .height(28.),
                text(792, body)
                    .font_size(13)
                    .fill(quiet())
                    .view()
                    .build(app)
                    .height(22.),
                input.height(38.),
                row_spaced(
                    8.,
                    vec![
                        action_button("Cancel", GuiAction::Cancel, false, app).width(82.),
                        action_button("Confirm", GuiAction::Confirm, true, app).width(92.),
                    ],
                )
                .height(34.),
            ],
        )
        .pad(14.),
    ])
}

fn adsr_editor<'a>(
    module: &'a crate::model::Module,
    selected: usize,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let attack = module
        .parameters()
        .get(2)
        .map(|parameter| parameter_fill(parameter.value()))
        .unwrap_or(0.);
    let sustain = module
        .parameters()
        .get(3)
        .map(|parameter| parameter_fill(parameter.value()))
        .unwrap_or(0.);
    row_spaced(
        10.,
        vec![
            meter_bar(810, "Attack", attack, selected == 0, app),
            meter_bar(820, "Sustain", sustain, selected == 1, app),
        ],
    )
}

fn envelope_editor<'a>(
    points: &'a [crate::model::EnvPoint],
    selected: usize,
    editing: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let graph_points = points.to_vec();
    let mut layers = vec![
        rect(840)
            .fill(field())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(6.)
            .build(app)
            .inert(),
        rect(842)
            .fill(Color::TRANSPARENT)
            .stroke(quiet(), Stroke::new(1.))
            .corner_rounding(3.)
            .build(app)
            .pad(10.)
            .inert(),
        path(841, move |area| envelope_path(area, &graph_points))
            .stroke(Color::from_rgb8(255, 200, 100), Stroke::new(2.))
            .build(app)
            .pad(10.),
    ];
    layers.extend(points.iter().enumerate().map(|(index, point)| {
        let x = (point.time as f32 / 100.).clamp(0., 1.);
        let y = (1. - ((point.value + 100) as f32 / 200.).clamp(0., 1.)).clamp(0., 1.);
        envelope_point(
            850 + index as u64,
            x,
            y,
            point.curve,
            selected == index,
            editing,
            app,
        )
    }));
    stack(layers).height_range(92.0..=150.0)
}

fn probe_editor<'a>(history: Vec<f32>, len: u32, app: &mut PaneState) -> View<'a, GuiState> {
    let current = history.last().copied().unwrap_or(0.0);
    let label = format!("{current:+.4}  {} samples", len);
    let graph = history;
    stack(vec![
        rect(900)
            .fill(field())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(6.)
            .build(app),
        rect(901)
            .fill(Color::TRANSPARENT)
            .stroke(quiet(), Stroke::new(1.))
            .corner_rounding(3.)
            .build(app)
            .pad(10.)
            .inert(),
        path(902, move |area| probe_path(area, &graph))
            .stroke(Color::from_rgb8(96, 210, 210), Stroke::new(2.))
            .build(app)
            .pad(10.),
        text(903, label)
            .font_size(13)
            .fill(fg())
            .view()
            .build(app)
            .pad_x(16.)
            .pad_y(12.),
    ])
    .height_range(92.0..=150.0)
}

fn sample_editor<'a>(zoom: u16, offset: u16, app: &mut PaneState) -> View<'a, GuiState> {
    let visible = (100 / zoom.max(1)).max(1) as f32 / 100.;
    let start = offset as f32 / 100.;
    stack(vec![
        rect(920)
            .fill(field())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(6.)
            .build(app),
        row_spaced(
            4.,
            (0..24)
                .map(|index| {
                    let height = 14. + ((index * 37) % 48) as f32;
                    rect(930 + index as u64)
                        .fill(Color::from_rgb8(80, 125, 202))
                        .corner_rounding(3.)
                        .build(app)
                        .width(7.)
                        .height(height)
                        .align(Align::Bottom)
                })
                .collect(),
        )
        .pad_x(16.)
        .pad_y(16.),
        column_spaced(
            6.,
            vec![
                space().height(58.),
                row_spaced(
                    8.,
                    vec![
                        meter_bar(960, "Offset", start, false, app),
                        meter_bar(970, "Window", visible, true, app),
                    ],
                )
                .height(24.),
            ],
        )
        .pad(10.),
    ])
}

fn meter_bar<'a>(
    id: u64,
    label: &'static str,
    value: f32,
    selected: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let value = value.clamp(0., 1.);
    stack(vec![
        rect(id)
            .fill(if selected {
                Color::from_rgb8(49, 70, 86)
            } else {
                field()
            })
            .stroke(if selected { accent() } else { line() }, Stroke::new(1.))
            .corner_rounding(6.)
            .build(app),
        rect(id + 1)
            .fill(accent().with_alpha(0.5))
            .corner_rounding(5.)
            .build(app)
            .width((118. * value).max(4.))
            .height(22.)
            .pad(6.),
        text(id + 2, label)
            .font_size(12)
            .fill(fg())
            .view()
            .build(app)
            .pad_x(10.)
            .pad_y(8.),
    ])
    .width(130.)
}

fn envelope_path(area: Area, points: &[crate::model::EnvPoint]) -> BezPath {
    let mut path = BezPath::new();
    if points.is_empty() {
        return path;
    }
    let width = area.width.max(1.);
    let height = area.height.max(1.);
    for index in 0..=48 {
        let time = index as f32 / 48.;
        let value = envelope_value(points, time);
        let x = area.x + time * width;
        let y = area.y + (1. - ((value + 1.) * 0.5)) * height;
        if index == 0 {
            path.move_to((x as f64, y as f64));
        } else {
            path.line_to((x as f64, y as f64));
        }
    }
    path
}

fn probe_path(area: Area, history: &[f32]) -> BezPath {
    let mut path = BezPath::new();
    if history.is_empty() {
        let y = area.y + area.height * 0.5;
        path.move_to((area.x as f64, y as f64));
        path.line_to(((area.x + area.width) as f64, y as f64));
        return path;
    }
    let min = history.iter().copied().fold(f32::INFINITY, f32::min);
    let max = history.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).abs().max(0.001);
    let width = area.width.max(1.);
    let height = area.height.max(1.);
    for (index, value) in history.iter().copied().enumerate() {
        let x = if history.len() <= 1 {
            area.x
        } else {
            area.x + index as f32 / (history.len() - 1) as f32 * width
        };
        let y = area.y + (1. - ((value - min) / range).clamp(0., 1.)) * height;
        if index == 0 {
            path.move_to((x as f64, y as f64));
        } else {
            path.line_to((x as f64, y as f64));
        }
    }
    path
}

fn envelope_value(points: &[crate::model::EnvPoint], time: f32) -> f32 {
    let time = time.clamp(0., 1.);
    let first = points[0];
    if points.len() == 1 || time <= first.time as f32 / 100. {
        return first.value as f32 / 100.;
    }
    let last = points[points.len() - 1];
    if time >= last.time as f32 / 100. {
        return last.value as f32 / 100.;
    }
    for pair in points.windows(2) {
        let start = pair[0];
        let end = pair[1];
        let start_time = start.time as f32 / 100.;
        let end_time = end.time as f32 / 100.;
        if time < start_time || time > end_time {
            continue;
        }
        let span = end_time - start_time;
        if span.abs() < f32::EPSILON {
            return start.value as f32 / 100.;
        }
        let position = (time - start_time) / span;
        let position = match (start.curve, end.curve) {
            (false, false) => position,
            (true, false) => 1. - (1. - position) * (1. - position),
            (false, true) => position * position,
            (true, true) if position < 0.5 => 2. * position * position,
            (true, true) => 1. - 2. * (1. - position) * (1. - position),
        };
        let start_value = start.value as f32 / 100.;
        let end_value = end.value as f32 / 100.;
        return start_value + (end_value - start_value) * position;
    }
    last.value as f32 / 100.
}

fn envelope_point<'a>(
    id: u64,
    x: f32,
    y: f32,
    curved: bool,
    selected: bool,
    editing: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let fill = if selected {
        if editing {
            fg()
        } else {
            accent()
        }
    } else if curved {
        Color::from_rgb8(255, 200, 100)
    } else {
        Color::from_rgb8(234, 238, 242)
    };
    path(id, move |area| envelope_point_path(area, x, y, curved, selected))
        .fill(fill)
        .stroke(field(), Stroke::new(if selected { 1.5 } else { 1. }))
        .build(app)
        .pad(10.)
}

fn envelope_point_path(area: Area, x: f32, y: f32, curved: bool, selected: bool) -> BezPath {
    let mut path = BezPath::new();
    let radius = if selected { 5. } else { 3.5 };
    let cx = area.x + x * area.width.max(1.);
    let cy = area.y + y * area.height.max(1.);
    if curved {
        path.move_to((cx as f64, (cy - radius) as f64));
        path.line_to(((cx + radius) as f64, cy as f64));
        path.line_to((cx as f64, (cy + radius) as f64));
        path.line_to(((cx - radius) as f64, cy as f64));
    } else {
        path.move_to(((cx - radius) as f64, (cy - radius) as f64));
        path.line_to(((cx + radius) as f64, (cy - radius) as f64));
        path.line_to(((cx + radius) as f64, (cy + radius) as f64));
        path.line_to(((cx - radius) as f64, (cy + radius) as f64));
    }
    path.close_path();
    path
}

fn parameter_fill(value: &ParameterValue) -> f32 {
    match value {
        ParameterValue::Float {
            value, min, max, ..
        } => ((*value - *min) as f32 / (*max - *min).max(1) as f32).clamp(0., 1.),
        ParameterValue::Int { value, min, max } => {
            ((*value - *min) as f32 / (*max - *min).max(1) as f32).clamp(0., 1.)
        }
        ParameterValue::Time { value, .. } => (*value as f32 / 10_000.).clamp(0., 1.),
        ParameterValue::Input => 1.,
        ParameterValue::Enum { index, options } => {
            if options.len() <= 1 {
                1.
            } else {
                *index as f32 / (options.len() - 1) as f32
            }
        }
        ParameterValue::Toggle(value) => f32::from(*value),
    }
}

fn grid_cell<'a>(
    state: &'a GuiState,
    position: GridPos,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = cell_id(position);
    let cursor = state.cursor() == position;
    let selected = state.selection().is_some_and(|(anchor, extent)| {
        let min_x = anchor.x.min(extent.x);
        let max_x = anchor.x.max(extent.x);
        let min_y = anchor.y.min(extent.y);
        let max_y = anchor.y.max(extent.y);
        position.x >= min_x && position.x <= max_x && position.y >= min_y && position.y <= max_y
    });

    let layers = vec![
        rect(id)
            .fill(if selected {
                Color::from_rgb8(38, 57, 56)
            } else {
                Color::from_rgb8(26, 29, 34)
            })
            .stroke(
                if cursor { accent() } else { line() },
                Stroke::new(if cursor { 2. } else { 1. }),
            )
            .corner_rounding(6.)
            .build(app),
    ];

    stack(layers).height(CELL).width(CELL)
}

fn module_tile<'a>(
    id: u64,
    module: &'a crate::model::Module,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let kind = module.kind();
    let mut code = kind.label().chars().take(3).collect::<String>();
    code.make_ascii_uppercase();
    let tile_width = module_span(module.width());
    let tile_height = module_span(module.height());
    let mut layers = vec![
        rect(id + 1)
            .fill(module_color(kind.category()))
            .stroke(Color::from_rgb8(10, 12, 14), Stroke::new(1.))
            .corner_rounding(6.)
            .build(app),
        rect(id + 9)
            .fill(Color::from_rgb8(8, 10, 12).with_alpha(0.42))
            .corner_rounding(5.)
            .build(app)
            .height(10.)
            .width(tile_width),
        text(id + 2, code)
            .font_size(8)
            .fill(Color::from_rgb8(248, 250, 252))
            .view()
            .build(app)
            .width(tile_width)
            .height(10.),
    ];

    if module.has_input_left() {
        for index in 0..module.input_count() {
            let port_id = if index == 0 && module.output_count() == 0 {
                id + 3
            } else {
                id + 30_000 + index as u64
            };
            layers.push(input_port(port_id, module.input_connected(index), app).offset(
                PORT_INSET,
                port_axis(index),
            ));
        }
    }

    if module.has_input_top() {
        for index in 0..module.input_count() {
            layers.push(
                input_port(id + 31_000 + index as u64, module.input_connected(index), app)
                    .offset(port_axis(index), PORT_INSET),
            );
        }
    }

    if module.has_output_right() {
        for index in 0..module.output_count() {
            let port_id = if module.output_count() == 1 {
                id + 4
            } else {
                id + 32_000 + index as u64
            };
            layers.push(
                output_port(port_id, app)
                    .offset(tile_width - PORT_INSET - PORT_SIZE, port_axis(index)),
            );
        }
    }

    if module.has_output_bottom() {
        for index in 0..module.output_count() {
            layers.push(
                output_port(id + 33_000 + index as u64, app)
                    .offset(port_axis(index), tile_height - PORT_INSET - PORT_SIZE),
            );
        }
    }

    stack_aligned(Align::TopLeading, layers)
        .width(tile_width)
        .height(tile_height)
}

fn input_port<'a>(id: u64, connected: bool, app: &mut PaneState) -> View<'a, GuiState> {
    if connected {
        circle(id)
            .fill(Color::from_rgb8(14, 16, 20))
            .stroke(Color::from_rgb8(248, 250, 252), Stroke::new(1.))
            .view()
            .build(app)
            .width(PORT_SIZE)
            .height(PORT_SIZE)
    } else {
        text(id + 5_000, "X")
            .font_size(11)
            .fill(Color::from_rgb8(248, 250, 252))
            .view()
            .build(app)
            .width(PORT_SIZE)
            .height(PORT_SIZE)
    }
}

fn output_port<'a>(id: u64, app: &mut PaneState) -> View<'a, GuiState> {
    circle(id)
        .fill(Color::from_rgb8(248, 250, 252))
        .view()
        .build(app)
        .width(PORT_SIZE)
        .height(PORT_SIZE)
}

fn module_span(cells: u16) -> f32 {
    cells as f32 * CELL + cells.saturating_sub(1) as f32 * GAP - TILE_PAD * 2.
}

fn port_axis(index: u16) -> f32 {
    index as f32 * (CELL + GAP) + CELL * 0.5 - TILE_PAD - PORT_SIZE * 0.5
}

fn grid_pointer_surface(app: &mut PaneState) -> View<'static, GuiState> {
    rect(990)
        .fill(Color::TRANSPARENT)
        .view()
        .gesture(gesture::click(991).button(MouseButton::Left).run(
            |state: &mut GuiState, _app, event| {
                if matches!(event.state, ClickPhase::Completed)
                    && let Some(position) = grid_position(event.location.local())
                {
                    state.click_grid_cell(position);
                }
            },
        ))
        .gesture(gesture::drag(992).button(MouseButton::Left).run(
            |state: &mut GuiState, _app, drag| match drag {
                DragPhase::Began { start, .. } => {
                    if let Some(position) = grid_position(start) {
                        state.drag_grid_cell(GridPointerPhase::Start, position);
                    }
                }
                DragPhase::Updated { current, .. } => {
                    if let Some(position) = grid_position(current) {
                        state.drag_grid_cell(GridPointerPhase::Drag, position);
                    }
                }
                DragPhase::Completed { current, .. } => {
                    if let Some(position) = grid_position(current) {
                        state.drag_grid_cell(GridPointerPhase::End, position);
                    }
                }
            },
        ))
        .build(app)
}

fn grid_position(point: Point) -> Option<GridPos> {
    if point.x < 0. || point.y < 0. || point.x >= grid_width() || point.y >= grid_height() {
        return None;
    }
    let x = (point.x / (CELL + GAP)).floor() as u16;
    let y = (point.y / (CELL + GAP)).floor() as u16;
    let cell_x = point.x - x as f32 * (CELL + GAP);
    let cell_y = point.y - y as f32 * (CELL + GAP);
    (x < GRID_COLUMNS && y < GRID_ROWS && cell_x < CELL && cell_y < CELL)
        .then_some(GridPos::new(x, y))
}

fn grid_width() -> f32 {
    GRID_COLUMNS as f32 * CELL + GRID_COLUMNS.saturating_sub(1) as f32 * GAP
}

fn grid_height() -> f32 {
    GRID_ROWS as f32 * CELL + GRID_ROWS.saturating_sub(1) as f32 * GAP
}

fn category_button<'a>(
    category: ModuleCategory,
    selected: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = 1_000 + category as u64;
    let color = module_color(category);
    stack(vec![
        rect(id)
            .fill(if selected {
                color
            } else {
                color.with_alpha(0.34)
            })
            .stroke(color, Stroke::new(if selected { 2. } else { 1. }))
            .corner_rounding(6.)
            .view()
            .gesture(gesture::click(id + 100).button(MouseButton::Left).run(
                move |state: &mut GuiState, _app, event| {
                    if matches!(event.state, ClickPhase::Completed) {
                        state.open_category(category);
                    }
                },
            ))
            .build(app)
            .width(row_width(category.label(), 13.))
            .height(PALETTE_ROW_HEIGHT),
        text(id + 200, category.label())
            .font_size(13)
            .fill(fg())
            .view()
            .build(app)
            .pad_x(10.)
            .pad_y(5.),
    ])
}

fn module_choice<'a>(
    index: usize,
    kind: ModuleKind,
    selected: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = 2_000 + index as u64;
    let color = module_color(kind.category());
    stack(vec![
        rect(id)
            .fill(if selected {
                color.with_alpha(0.74)
            } else {
                color.with_alpha(0.22)
            })
            .stroke(if selected { color } else { line() }, Stroke::new(1.))
            .corner_rounding(6.)
            .view()
            .gesture(gesture::click(id + 100).button(MouseButton::Left).run(
                move |state: &mut GuiState, _app, event| {
                    if matches!(event.state, ClickPhase::Completed) {
                        state.choose_palette_index(index);
                    }
                },
            ))
            .build(app)
            .width(row_width(kind.label(), 13.))
            .height(PALETTE_ROW_HEIGHT),
        text(id + 200, kind.label())
            .font_size(13)
            .fill(fg())
            .view()
            .build(app)
            .pad_x(10.)
            .pad_y(5.),
    ])
}

fn filtered_module_choice<'a>(
    index: usize,
    kind: ModuleKind,
    selected: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = 2_500 + index as u64;
    let color = module_color(kind.category());
    stack(vec![
        rect(id)
            .fill(if selected {
                color.with_alpha(0.74)
            } else {
                color.with_alpha(0.22)
            })
            .stroke(if selected { color } else { line() }, Stroke::new(1.))
            .corner_rounding(6.)
            .view()
            .gesture(gesture::click(id + 100).button(MouseButton::Left).run(
                move |state: &mut GuiState, _app, event| {
                    if matches!(event.state, ClickPhase::Completed) {
                        state.choose_filtered_palette_index(index);
                        state.apply(GuiAction::Confirm);
                    }
                },
            ))
            .build(app)
            .width(row_width(kind.label(), 13.))
            .height(PALETTE_ROW_HEIGHT),
        text(id + 200, kind.label())
            .font_size(13)
            .fill(fg())
            .view()
            .build(app)
            .pad_x(10.)
            .pad_y(5.),
    ])
}

fn action_button<'a>(
    label: &'static str,
    action: GuiAction,
    active: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = 3_000 + action_id(action);
    stack(vec![
        rect(id)
            .fill(if active { accent() } else { field() })
            .stroke(if active { accent() } else { line() }, Stroke::new(1.))
            .corner_rounding(7.)
            .view()
            .gesture(gesture::click(id + 100).button(MouseButton::Left).run(
                move |state: &mut GuiState, _app, event| {
                    if matches!(event.state, ClickPhase::Completed) {
                        state.apply(action);
                    }
                },
            ))
            .build(app),
        text(id + 200, label)
            .font_size(13)
            .fill(if active { Color::BLACK } else { fg() })
            .view()
            .build(app)
            .pad_x(9.)
            .pad_y(6.),
    ])
    .height(28.)
}

fn status_chip<'a>(label: String, color: Color, app: &mut PaneState) -> View<'a, GuiState> {
    let id = 4_000 + label.len() as u64;
    stack(vec![
        rect(id)
            .fill(color)
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(7.)
            .build(app),
        text(id + 1, label)
            .font_size(13)
            .fill(fg())
            .view()
            .build(app)
            .pad_x(10.)
            .pad_y(6.),
    ])
}

fn shortcuts(state: &GuiState) -> &'static [(&'static str, &'static str)] {
    match state.mode() {
        Mode::Normal => &[
            ("n", "module"),
            ("i", "edit"),
            ("m", "move"),
            (",", "select"),
            ("space", "play"),
            ("O", "load"),
            ("s", "settings"),
        ],
        Mode::Palette => &[
            ("hjkl", "choose"),
            ("! ... *", "category"),
            ("enter", "place"),
            ("/", "search"),
            ("i esc", "close"),
        ],
        Mode::Move { .. } | Mode::Copy { .. } => &[
            ("hjkl", "move"),
            ("enter", "place"),
            ("o", "rotate"),
            ("p", "subpatch"),
            ("esc", "cancel"),
        ],
        Mode::Edit { .. } => &[
            ("jk/hl", "param/value"),
            ("t", "type"),
            (";/u/s", "port/unit/step"),
            ("space", "play"),
            ("i esc", "done"),
        ],
        Mode::AdsrEdit { .. } => &[("jk/hl", "shape"), ("space", "play"), ("i esc", "done")],
        Mode::EnvEdit { editing: true, .. } => &[
            ("hjkl", "move"),
            ("s", "step"),
            ("space", "play"),
            ("m enter", "done"),
            ("esc", "cancel"),
        ],
        Mode::EnvEdit { .. } => &[
            ("hl", "point"),
            ("m", "move"),
            ("n/.", "add/delete"),
            ("c/s", "curve/step"),
            ("space", "play"),
            ("i esc", "done"),
        ],
        Mode::ProbeEdit { .. } => &[
            ("hl/jk", "length"),
            ("r", "reset"),
            ("space", "play"),
            ("i esc", "done"),
        ],
        Mode::SampleView { .. } => &[
            ("hl", "pan"),
            ("jk", "zoom"),
            ("r", "reset"),
            ("space", "play"),
            ("i esc", "done"),
        ],
        Mode::Select { .. } => &[
            ("hjkl", "resize"),
            ("enter", "move"),
            ("y", "copy"),
            (".", "delete"),
            (", esc", "cancel"),
        ],
        Mode::SelectMove { .. } | Mode::CopySelection { .. } => {
            &[("hjkl", "move"), ("enter", "place"), ("esc", "cancel")]
        }
        Mode::ValueInput { .. }
        | Mode::SavePrompt
        | Mode::ExportPrompt
        | Mode::TrackPrompt => {
            &[("type", "text"), ("enter", "confirm"), ("esc", "cancel")]
        }
        Mode::SaveConfirm | Mode::ExportConfirm => &[("Y enter", "overwrite"), ("N esc", "back")],
        Mode::TrackSettings { .. } => &[
            ("jk", "field"),
            ("hl", "value"),
            ("space", "play"),
            ("enter", "confirm"),
            ("esc", "cancel"),
        ],
        Mode::QuitConfirm => &[("Y", "quit"), ("N esc", "cancel")],
    }
}

fn shortcut_chip<'a>(
    index: usize,
    key: &'static str,
    label: &'static str,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = 70_000 + index as u64 * 10;
    stack(vec![
        rect(id)
            .fill(field())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(6.)
            .build(app),
        row_spaced(
            5.,
            vec![
                text(id + 1, key)
                    .font_size(11)
                    .fill(accent())
                    .view()
                    .build(app),
                text(id + 2, label)
                    .font_size(11)
                    .fill(fg())
                    .view()
                    .build(app),
            ],
        )
        .pad_x(7.)
        .pad_y(5.),
    ])
}

fn key(input: &'static str, action: GuiAction) -> Gesture<GuiState> {
    gesture::key(ROOT + action_id(action))
        .key(Key::character(input))
        .run(move |state: &mut GuiState, _, event| {
            if event.phase == KeyPhase::Pressed {
                if native_text_prompt(state) {
                    return;
                }
                if state.text_input_active()
                    && let Some(character) = single_character(input)
                {
                    state.apply(GuiAction::InputChar(character));
                    return;
                }
                state.apply(action);
            }
        })
}

fn text_input_keys(app: &mut PaneState) -> View<'static, GuiState> {
    stack(
        r#"abdfgqxzABCDEFGIMOPRSTVXZ0123456789-_+#!"$%&'()*:<=>?@\^`{}|~"#
            .chars()
            .map(|character| {
                rect(8_000 + character as u64)
                    .fill(Color::TRANSPARENT)
                    .view()
                    .gesture(input_char_key(character))
                    .build(app)
            })
            .collect(),
    )
}

fn input_char_key(character: char) -> Gesture<GuiState> {
    gesture::key(8_500 + character as u64)
        .key(Key::character(character.to_string()))
        .run(move |state: &mut GuiState, _, event| {
            if event.phase == KeyPhase::Pressed {
                if native_text_prompt(state) {
                    return;
                }
                let action = if state.text_input_active() || state.palette_searching() {
                    GuiAction::InputChar(character)
                } else if let Some(category) = category_key(character) {
                    GuiAction::Palette(category)
                } else {
                    GuiAction::InputChar(character)
                };
                state.apply(action);
            }
        })
}

fn category_key(character: char) -> Option<ModuleCategory> {
    match character {
        '!' => Some(ModuleCategory::Source),
        '@' => Some(ModuleCategory::Shape),
        '#' => Some(ModuleCategory::Filter),
        '$' => Some(ModuleCategory::Effect),
        '%' => Some(ModuleCategory::Logic),
        '^' => Some(ModuleCategory::Routing),
        '&' => Some(ModuleCategory::Subpatch),
        '*' => Some(ModuleCategory::Output),
        _ => None,
    }
}

fn edit_key(
    input: &'static str,
    normal_action: GuiAction,
    edit_action: GuiAction,
) -> Gesture<GuiState> {
    gesture::key(ROOT + action_id(normal_action) + action_id(edit_action))
        .key(Key::character(input))
        .run(move |state: &mut GuiState, _, event| {
            if event.phase == KeyPhase::Pressed {
                if native_text_prompt(state) {
                    return;
                }
                if state.text_input_active()
                    && let Some(character) = single_character(input)
                {
                    state.apply(GuiAction::InputChar(character));
                    return;
                }
                let action = if matches!(state.mode(), Mode::Edit { .. }) {
                    edit_action
                } else {
                    normal_action
                };
                state.apply(action);
            }
        })
}

fn step_key() -> Gesture<GuiState> {
    gesture::key(ROOT + 1_900).key(Key::character("s")).run(
        move |state: &mut GuiState, _, event| {
            if event.phase == KeyPhase::Pressed {
                if native_text_prompt(state) {
                    return;
                }
                if state.text_input_active() {
                    state.apply(GuiAction::InputChar('s'));
                    return;
                }
                let action = if matches!(state.mode(), Mode::Edit { .. } | Mode::EnvEdit { .. }) {
                    GuiAction::CycleStep
                } else {
                    GuiAction::TrackSettings
                };
                state.apply(action);
            }
        },
    )
}

fn yes_key(input: &'static str, fallback: GuiAction) -> Gesture<GuiState> {
    gesture::key(ROOT + action_id(fallback) + 2_000)
        .key(Key::character(input))
        .run(move |state: &mut GuiState, _, event| {
            if event.phase == KeyPhase::Pressed {
                if native_text_prompt(state) {
                    return;
                }
                if state.text_input_active()
                    && let Some(character) = single_character(input)
                {
                    state.apply(GuiAction::InputChar(character));
                    return;
                }
                let action = if confirmation_mode(state) {
                    GuiAction::Confirm
                } else {
                    fallback
                };
                state.apply(action);
            }
        })
}

fn new_or_no_key() -> Gesture<GuiState> {
    gesture::key(ROOT + 2_100).key(Key::character("n")).run(
        move |state: &mut GuiState, _, event| {
            if event.phase == KeyPhase::Pressed {
                if native_text_prompt(state) {
                    return;
                }
                if state.text_input_active() {
                    state.apply(GuiAction::InputChar('n'));
                } else if confirmation_mode(state) {
                    state.apply(GuiAction::Cancel);
                } else if matches!(state.mode(), Mode::EnvEdit { .. }) {
                    state.apply(GuiAction::AddPoint);
                } else {
                    state.apply(GuiAction::OpenPalette);
                }
            }
        },
    )
}

fn confirmation_mode(state: &GuiState) -> bool {
    matches!(
        state.mode(),
        Mode::QuitConfirm | Mode::SaveConfirm | Mode::ExportConfirm
    )
}

fn env_key(
    input: &'static str,
    normal_action: GuiAction,
    env_action: GuiAction,
) -> Gesture<GuiState> {
    gesture::key(ROOT + action_id(normal_action) + action_id(env_action) + 1_000)
        .key(Key::character(input))
        .run(move |state: &mut GuiState, _, event| {
            if event.phase == KeyPhase::Pressed {
                if native_text_prompt(state) {
                    return;
                }
                if state.text_input_active()
                    && let Some(character) = single_character(input)
                {
                    state.apply(GuiAction::InputChar(character));
                    return;
                }
                let action = if matches!(state.mode(), Mode::EnvEdit { .. }) {
                    env_action
                } else {
                    normal_action
                };
                state.apply(action);
            }
        })
}

fn single_character(input: &str) -> Option<char> {
    let mut chars = input.chars();
    let character = chars.next()?;
    chars.next().is_none().then_some(character)
}

fn native_text_prompt(state: &GuiState) -> bool {
    matches!(state.mode(), Mode::TrackPrompt)
}

fn key_named(input: NamedKey, action: GuiAction) -> Gesture<GuiState> {
    gesture::key(ROOT + action_id(action) + 100).key(input).run(
        move |state: &mut GuiState, _, event| {
            if event.phase == KeyPhase::Pressed {
                if native_text_prompt(state)
                    && !matches!(input, NamedKey::Enter | NamedKey::Escape)
                {
                    return;
                }
                if input == NamedKey::Space
                    && matches!(
                        state.mode(),
                        Mode::SavePrompt | Mode::ExportPrompt
                    )
                {
                    state.apply(GuiAction::InputChar(' '));
                    return;
                }
                state.apply(action);
            }
        },
    )
}

fn action_id(action: GuiAction) -> u64 {
    match action {
        GuiAction::Left => 10,
        GuiAction::Down => 11,
        GuiAction::Up => 12,
        GuiAction::Right => 13,
        GuiAction::LeftFast => 14,
        GuiAction::DownFast => 15,
        GuiAction::UpFast => 16,
        GuiAction::RightFast => 17,
        GuiAction::OpenPalette => 18,
        GuiAction::TogglePlay => 19,
        GuiAction::ToggleMeters => 20,
        GuiAction::Load => 58,
        GuiAction::Quit => 59,
        GuiAction::Save => 60,
        GuiAction::SaveAs => 61,
        GuiAction::Export => 62,
        GuiAction::TrackSettings => 67,
        GuiAction::TrackEdit => 68,
        GuiAction::Search => 69,
        GuiAction::EditSubpatch => 70,
        GuiAction::ExitSubpatch => 71,
        GuiAction::Palette(category) => 1_100 + category_index(category) as u64,
        GuiAction::PaletteLeft => 21,
        GuiAction::PaletteRight => 22,
        GuiAction::PaletteUp => 23,
        GuiAction::PaletteDown => 24,
        GuiAction::Confirm => 25,
        GuiAction::Cancel => 26,
        GuiAction::Delete => 27,
        GuiAction::Edit => 28,
        GuiAction::Move => 29,
        GuiAction::Copy => 30,
        GuiAction::Rotate => 31,
        GuiAction::Select => 32,
        GuiAction::Undo => 33,
        GuiAction::Redo => 34,
        GuiAction::Instrument(index) => 35 + index as u64,
        GuiAction::NewInstrument => 45,
        GuiAction::HelpScrollUp => 46,
        GuiAction::HelpScrollDown => 47,
        GuiAction::ValueDown => 48,
        GuiAction::ValueUp => 49,
        GuiAction::ValueDownFast => 50,
        GuiAction::ValueUpFast => 51,
        GuiAction::TogglePort => 52,
        GuiAction::CycleUnit => 53,
        GuiAction::CycleStep => 54,
        GuiAction::TypeValue => 55,
        GuiAction::InputChar(character) => 1_000 + character as u64,
        GuiAction::Backspace => 63,
        GuiAction::DeleteChar => 64,
        GuiAction::TextStart => 65,
        GuiAction::TextEnd => 66,
        GuiAction::AddPoint => 56,
        GuiAction::DeletePoint => 57,
        GuiAction::ToggleCurve => 58,
    }
}

fn category_index(category: ModuleCategory) -> usize {
    ModuleCategory::ALL
        .iter()
        .position(|candidate| *candidate == category)
        .unwrap_or(0)
}

fn status_text(state: &GuiState) -> String {
    let cursor = state.cursor();
    format!(
        "{} x:{} y:{} {}",
        if state.playing() { "Playing" } else { "Stopped" },
        cursor.x,
        cursor.y,
        state.audio_status()
    )
}

fn mode_text(state: &GuiState) -> String {
    match state.mode() {
        Mode::Normal => "Normal".to_string(),
        Mode::QuitConfirm => "Quit confirm".to_string(),
        Mode::Palette => {
            if state.palette_searching() {
                format!("Search {}", state.palette_filter())
            } else {
                format!(
                    "{} / {}",
                    state.palette_category().label(),
                    state.selected_palette_module().label()
                )
            }
        }
        Mode::Move { module, .. } => format!("Moving {}", module.value()),
        Mode::Copy { source } => format!("Copying {}", source.value()),
        Mode::Edit { module, parameter } => {
            format!("Edit {} param {}", module.value(), parameter + 1)
        }
        Mode::ValueInput { module, parameter } => {
            format!("Input {} param {}", module.value(), parameter + 1)
        }
        Mode::AdsrEdit { module, parameter } => {
            format!("ADSR {} param {}", module.value(), parameter + 1)
        }
        Mode::EnvEdit {
            module,
            point,
            editing,
        } => format!(
            "Envelope {} point {} {}",
            module.value(),
            point + 1,
            if editing { "move" } else { "select" }
        ),
        Mode::ProbeEdit { module } => format!("Probe {} view", module.value()),
        Mode::SampleView {
            module,
            zoom,
            offset,
        } => format!("Sample {} zoom {} offset {}", module.value(), zoom, offset),
        Mode::Select { anchor } => format!("Select {}:{}", anchor.x, anchor.y),
        Mode::SelectMove { origin, .. } => format!("Move selection {}:{}", origin.x, origin.y),
        Mode::CopySelection { origin, .. } => format!("Copy selection {}:{}", origin.x, origin.y),
        Mode::SavePrompt => "Save".to_string(),
        Mode::SaveConfirm => "Overwrite save".to_string(),
        Mode::ExportPrompt => "Export".to_string(),
        Mode::ExportConfirm => "Overwrite export".to_string(),
        Mode::TrackPrompt => "Track".to_string(),
        Mode::TrackSettings { parameter } => format!("Settings {}", parameter + 1),
    }
}

fn cell_id(position: GridPos) -> u64 {
    10_000 + position.y as u64 * 100 + position.x as u64 * 10
}

fn module_color(category: ModuleCategory) -> Color {
    match category {
        ModuleCategory::Source => Color::from_rgb8(34, 139, 132),
        ModuleCategory::Shape => Color::from_rgb8(188, 112, 42),
        ModuleCategory::Filter => Color::from_rgb8(80, 125, 202),
        ModuleCategory::Effect => Color::from_rgb8(154, 86, 178),
        ModuleCategory::Logic => Color::from_rgb8(190, 76, 91),
        ModuleCategory::Routing => Color::from_rgb8(110, 128, 84),
        ModuleCategory::Subpatch => Color::from_rgb8(96, 112, 140),
        ModuleCategory::Output => Color::from_rgb8(214, 171, 68),
    }
}

fn bg() -> Color {
    Color::from_rgb8(12, 14, 18)
}

fn panel() -> Color {
    Color::from_rgb8(24, 27, 33)
}

fn field() -> Color {
    Color::from_rgb8(32, 36, 43)
}

fn line() -> Color {
    Color::from_rgb8(61, 68, 78)
}

fn fg() -> Color {
    Color::from_rgb8(234, 238, 242)
}

fn quiet() -> Color {
    Color::from_rgb8(142, 151, 164)
}

fn accent() -> Color {
    Color::from_rgb8(73, 205, 180)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_connection_segment_uses_port_edges() {
        assert_eq!(
            connection_segment(GridPos::new(0, 0), Orientation::Right, GridPos::new(2, 0)),
            WireSegment {
                x: 22.5,
                y: 13.,
                width: 51.,
                height: 4.,
            }
        );
    }

    #[test]
    fn vertical_connection_segment_uses_port_edges() {
        assert_eq!(
            connection_segment(GridPos::new(0, 0), Orientation::Down, GridPos::new(0, 2)),
            WireSegment {
                x: 13.,
                y: 22.5,
                width: 4.,
                height: 51.,
            }
        );
    }
}
