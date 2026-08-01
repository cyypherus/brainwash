use crate::model::{
    EnvPointerPhase, GridPointerPhase, GridPos, GridRenderModule, GridViewSize, GuiAction,
    GuiState, Mode, Module, ModuleCategory, ModuleId, ModuleKind, Orientation, PaletteModule,
    ParameterValue,
};
use brainwash_grid::bounded_delta;
use haven::*;

const ROOT: u64 = 1;
const CELL: f32 = 30.;
const GAP: f32 = 3.;
const TILE_PAD: f32 = 3.;
const PORT_SIZE: f32 = 7.;
const PORT_INSET: f32 = 1.;
const METER_SIZE: f32 = 13.;
const WIRE_THICKNESS: f32 = 4.;
const PALETTE_ROW_HEIGHT: f32 = 23.;
const TITLE_BAR_SPACE: f32 = 40.;
const PANEL_GAP: f32 = 10.;
const ENVELOPE_VALUE_MIN: f32 = -1.;
const ENVELOPE_VALUE_MAX: f32 = 1.;
const PREVIEW_ID_OFFSET: u64 = 100_000;
const PALETTE_ICON_SIZE: f32 = 17.;

pub fn main_view<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let side_panel = column_spaced(
        PANEL_GAP,
        vec![document_bar(state, app), shortcut_panel(state, app)],
    );

    let mut layers = vec![
        row_spaced_aligned(
            PANEL_GAP,
            Align::TopLeading,
            vec![
                column_spaced(
                    PANEL_GAP,
                    vec![toolbar(state, app), patch_panel(state, app).expand()],
                )
                .expand(),
                side_panel,
            ],
        )
        .pad(PANEL_GAP)
        .expand(),
    ];
    if matches!(
        state.mode(),
        Mode::QuitConfirm
            | Mode::ValueInput { .. }
            | Mode::LoadConfirm
            | Mode::SaveConfirm
            | Mode::ExportPrompt
            | Mode::ExportConfirm
            | Mode::TrackPrompt
            | Mode::TrackSettings { .. }
    ) {
        let prompt_height = if matches!(state.mode(), Mode::LoadConfirm | Mode::SaveConfirm) {
            132.
        } else {
            178.
        };
        layers.push(
            prompt_panel(state, app)
                .width(360.)
                .height(prompt_height)
                .align(Align::CenterCenter),
        );
    }
    let mut root_layers = vec![
        key_surface(app).expand(),
        stack(layers).pad_top(TITLE_BAR_SPACE).expand(),
    ];
    if matches!(state.mode(), Mode::Palette) || state.palette_searching() {
        root_layers.push(
            column_aligned(
                Align::TopCenter,
                vec![space().height(TITLE_BAR_SPACE), palette_panel(state, app)],
            )
            .expand(),
        );
    }
    stack(root_layers).align(Align::TopLeading)
}

fn key_surface(app: &mut PaneState) -> View<'static, GuiState> {
    stack(vec![
        rect(ROOT).fill(bg()).build(app).expand(),
        binding_surface(app, None).expand(),
    ])
}

fn binding_surface(
    app: &mut PaneState,
    grid_size: Option<GridViewSize>,
) -> View<'static, GuiState> {
    let mut layers = Vec::new();
    layers.extend(
        binding_inputs()
            .into_iter()
            .enumerate()
            .map(|(index, input)| {
                rect(8_000 + index as u64)
                    .fill(Color::TRANSPARENT)
                    .view()
                    .gesture(binding_key(8_500 + index as u64, input, grid_size))
                    .build(app)
                    .expand()
            }),
    );
    stack(layers)
}

fn toolbar<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let mut actions = vec![
        action_button(
            binding!(state.play_button),
            if state.playing() { "Pause" } else { "Play" },
            GuiAction::TogglePlay,
            state.playing(),
            app,
        )
        .width(50.),
        action_button(
            binding!(state.meters_button),
            "Meters",
            GuiAction::ToggleMeters,
            state.show_meters(),
            app,
        ),
        action_button(
            binding!(state.load_button),
            "Open",
            GuiAction::Load,
            false,
            app,
        ),
        action_button(
            binding!(state.save_button),
            "Save",
            GuiAction::Save,
            false,
            app,
        ),
    ];
    if state.composition_depth() > 0 && matches!(state.mode(), Mode::Normal) {
        actions.push(action_button(
            binding!(state.save_module_button),
            "Save Module",
            GuiAction::SaveModule,
            false,
            app,
        ));
    }
    actions.extend([
        action_button(
            binding!(state.export_button),
            "Export",
            GuiAction::Export,
            false,
            app,
        ),
        action_button(
            binding!(state.modules_button),
            "Modules",
            GuiAction::OpenModules,
            false,
            app,
        ),
        action_button(
            binding!(state.track_button),
            "Track",
            GuiAction::TrackEdit,
            false,
            app,
        ),
        status_chip(
            format!(
                "{} / {}",
                state.active_instrument() + 1,
                state.instrument_count()
            ),
            panel(),
            app,
        ),
    ]);
    row_spaced_aligned(PANEL_GAP, Align::TopLeading, actions)
}

fn document_bar<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let document_color = if state.dirty() {
        Color::from_rgb8(255, 200, 100)
    } else {
        fg()
    };
    let status_color = if state.document_status().contains("failed") {
        Color::from_rgb8(255, 120, 120)
    } else {
        quiet()
    };
    row_spaced(
        PANEL_GAP,
        vec![
            text(4_100, state.document_label())
                .font_size(12)
                .fill(document_color)
                .view()
                .build(app)
                .pad_x(2.),
            text(4_101, state.document_status())
                .font_size(12)
                .fill(status_color)
                .view()
                .build(app)
                .pad_x(2.),
        ],
    )
    .height(20.)
}

fn palette_panel<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let (width, height) = palette_panel_size(state);
    let content_width = width - 24.;
    let content = if state.palette_searching() {
        let filter = state.palette_filter().to_string();
        let modules = state
            .filtered_palette_modules()
            .iter()
            .enumerate()
            .map(|(index, module)| {
                filtered_module_choice(
                    index,
                    module,
                    Some(module.kind()) == state.selected_filtered_palette_module(),
                    content_width,
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
            ])
            .width(content_width),
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
            .map(|(index, module)| {
                let selected = state.mode() == Mode::Palette
                    && module.kind() == state.selected_palette_module();
                module_choice(index, module, selected, content_width, app)
            })
            .collect::<Vec<_>>();
        vec![row_spaced(6., categories), column_spaced(6., modules)]
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
        let width = title_w
            .max(module_w + PANEL_PAD)
            .max(category_w + PANEL_PAD);
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
    let parameter_edit = matches!(state.mode(), Mode::Edit { .. } | Mode::ValueInput { .. });
    let align = if parameter_edit {
        Align::CenterCenter
    } else {
        Align::TopLeading
    };
    let content = match state.mode() {
        Mode::Edit { .. } | Mode::ValueInput { .. } => edit_panel(state, app).pad(PANEL_GAP),
        Mode::AdsrEdit { .. }
        | Mode::EnvEdit { .. }
        | Mode::ProbeEdit { .. }
        | Mode::SampleView { .. } => edit_panel(state, app).expand().pad(PANEL_GAP),
        _ => patch_grid(state).pad(PANEL_GAP).expand(),
    };
    stack_aligned(
        align,
        vec![
            rect(300)
                .fill(Color::from_rgb8(18, 20, 24))
                .stroke(mode_color(state), Stroke::new(1.5))
                .corner_rounding(8.)
                .build(app),
            content,
        ],
    )
}

fn mode_overlay<'a>(state: &GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let label = mode_text(state).to_ascii_uppercase();
    let width = text_width(&label, 11.) + 16.;
    stack(vec![
        rect(301)
            .fill(Color::from_rgb8(8, 10, 12).with_alpha(0.76))
            .stroke(mode_color(state), Stroke::new(1.))
            .corner_rounding(4.)
            .build(app)
            .inert(),
        text(302, label)
            .font_size(11)
            .fill(fg())
            .view()
            .build(app)
            .pad_x(8.)
            .pad_y(5.),
    ])
    .width(width)
    .height(24.)
    .inert()
}

fn mode_color(state: &GuiState) -> Color {
    match state.mode() {
        Mode::Normal => line(),
        Mode::Palette => module_color(state.palette_category()),
        Mode::Move { .. }
        | Mode::Copy { .. }
        | Mode::Select { .. }
        | Mode::SelectMove { .. }
        | Mode::CopySelection { .. } => Color::from_rgb8(255, 200, 100),
        Mode::Edit { .. }
        | Mode::ValueInput { .. }
        | Mode::AdsrEdit { .. }
        | Mode::EnvEdit { .. }
        | Mode::ProbeEdit { .. }
        | Mode::SampleView { .. }
        | Mode::TrackSettings { .. } => accent(),
        Mode::QuitConfirm
        | Mode::LoadConfirm
        | Mode::SaveConfirm
        | Mode::ExportPrompt
        | Mode::ExportConfirm
        | Mode::TrackPrompt => Color::from_rgb8(190, 76, 91),
    }
}

fn patch_grid<'a>(state: &'a GuiState) -> View<'a, GuiState> {
    draw(move |area, app: &mut PaneState| grid_view(state, app, grid_view_size(state, area), area))
        .expand()
        .clipped(rect_path)
}

fn grid_view<'a>(
    state: &'a GuiState,
    app: &mut PaneState,
    size: GridViewSize,
    area: Area,
) -> Vec<PaneElement<GuiState>> {
    let preview = grid_preview(state);
    let view = state.grid_view_offset_for_size(size);
    let pixel_x = view.x as f32 * (CELL + GAP);
    let pixel_y = view.y as f32 * (CELL + GAP);
    let (inset_x, inset_y) = grid_inset(area, size);
    let hidden = preview
        .filter(|preview| !preview.copy)
        .map(|preview| {
            if let Some(module) = preview.source_module {
                vec![module]
            } else {
                modules_in_rect(state, preview.source_min, preview.source_max)
            }
        })
        .unwrap_or_default();
    let mut layers = vec![
        grid_content(
            state,
            app,
            0,
            &hidden,
            preview.is_none_or(|preview| preview.copy),
            view,
            size,
        )
        .offset(inset_x - pixel_x, inset_y - pixel_y),
    ];
    if let Some(preview) = preview {
        layers.push(grid_preview_layer(
            state,
            app,
            preview,
            pixel_x - inset_x,
            pixel_y - inset_y,
            area.width,
            area.height,
        ));
    }
    layers.push(mode_overlay(state, app).offset(6., (area.height - 30.).max(0.)));
    layers.push(
        grid_pointer_surface(app, view, size, area)
            .width(area.width)
            .height(area.height),
    );
    layers.push(binding_surface(app, Some(size)).expand());
    stack_aligned(Align::TopLeading, layers)
        .width(area.width)
        .height(area.height)
        .draw(area, app)
}

fn grid_content<'a>(
    state: &'a GuiState,
    app: &mut PaneState,
    id_offset: u64,
    hidden: &[ModuleId],
    show_selection: bool,
    view: GridPos,
    size: GridViewSize,
) -> View<'a, GuiState> {
    let (columns, rows) = state.grid_size();
    let visible_x = visible_axis(view.x, size.columns(), columns);
    let visible_y = visible_axis(view.y, size.rows(), rows);
    let row_views = visible_y
        .map(|y| {
            let cells = visible_x
                .clone()
                .map(|x| grid_cell(state, GridPos::new(x, y), id_offset, show_selection, app))
                .collect::<Vec<_>>();
            row_spaced(GAP, cells).height(CELL)
        })
        .collect::<Vec<_>>();
    let cursor = state.cursor();
    stack_aligned(
        Align::TopLeading,
        vec![
            column_spaced(GAP, row_views)
                .offset(view.x as f32 * (CELL + GAP), view.y as f32 * (CELL + GAP)),
            connection_layer(state, app, id_offset, hidden, view, size),
            module_layer(state, app, id_offset, hidden, view, size),
            rect(id_offset + 72_000)
                .fill(Color::TRANSPARENT)
                .stroke(Color::from_rgb8(248, 250, 252), Stroke::new(3.))
                .corner_rounding(6.)
                .build(app)
                .width(CELL)
                .height(CELL)
                .offset(
                    cursor.x as f32 * (CELL + GAP),
                    cursor.y as f32 * (CELL + GAP),
                ),
        ],
    )
    .width(grid_span(columns))
    .height(grid_span(rows))
    .inert()
}

fn visible_axis(origin: u16, visible: u16, total: u16) -> std::ops::Range<u16> {
    origin.min(total)..origin.saturating_add(visible).min(total)
}

#[derive(Clone, Copy)]
struct GridPreview {
    source_module: Option<ModuleId>,
    source_min: GridPos,
    source_max: GridPos,
    dx: i16,
    dy: i16,
    copy: bool,
}

fn grid_preview(state: &GuiState) -> Option<GridPreview> {
    match state.mode() {
        Mode::Copy { source } => {
            let module = state
                .modules()
                .iter()
                .find(|module| module.id() == source)?;
            let render_module = state.grid_render_module(module);
            let width = render_module.width.saturating_sub(1);
            let height = render_module.height.saturating_sub(1);
            Some(GridPreview {
                source_module: Some(source),
                source_min: module.position(),
                source_max: GridPos::new(module.position().x + width, module.position().y + height),
                dx: bounded_delta(
                    module.position().x,
                    module.position().x + width,
                    state.cursor().x as i16 - module.position().x as i16,
                    state.grid_size().0,
                ),
                dy: bounded_delta(
                    module.position().y,
                    module.position().y + height,
                    state.cursor().y as i16 - module.position().y as i16,
                    state.grid_size().1,
                ),
                copy: true,
            })
        }
        Mode::Move { module, origin } => {
            let module = state.moving_module(module)?;
            let render_module = state.grid_render_module(module);
            let width = render_module.width.saturating_sub(1);
            let height = render_module.height.saturating_sub(1);
            Some(GridPreview {
                source_module: Some(module.id()),
                source_min: module.position(),
                source_max: GridPos::new(module.position().x + width, module.position().y + height),
                dx: bounded_delta(
                    module.position().x,
                    module.position().x + width,
                    state.cursor().x as i16 - origin.x as i16,
                    state.grid_size().0,
                ),
                dy: bounded_delta(
                    module.position().y,
                    module.position().y + height,
                    state.cursor().y as i16 - origin.y as i16,
                    state.grid_size().1,
                ),
                copy: false,
            })
        }
        Mode::SelectMove {
            anchor,
            extent,
            origin,
        }
        | Mode::CopySelection {
            anchor,
            extent,
            origin,
        } => {
            let source = crate::model::GridRect::from_points(anchor, extent);
            let source_min = source.min();
            let source_max = source.max();
            Some(GridPreview {
                source_module: None,
                source_min,
                source_max,
                dx: bounded_delta(
                    source_min.x,
                    source_max.x,
                    state.cursor().x as i16 - origin.x as i16,
                    state.grid_size().0,
                ),
                dy: bounded_delta(
                    source_min.y,
                    source_max.y,
                    state.cursor().y as i16 - origin.y as i16,
                    state.grid_size().1,
                ),
                copy: matches!(state.mode(), Mode::CopySelection { .. }),
            })
        }
        _ => None,
    }
}

fn grid_preview_layer<'a>(
    state: &'a GuiState,
    app: &mut PaneState,
    preview: GridPreview,
    view_x: f32,
    view_y: f32,
    width: f32,
    height: f32,
) -> View<'a, GuiState> {
    let modules = if let Some(source_module) = preview.source_module {
        state
            .moving_module(source_module)
            .map(|module| state.grid_render_module(module))
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        state
            .grid_render_modules()
            .into_iter()
            .filter(|module| module_overlaps_rect(module, preview.source_min, preview.source_max))
            .collect::<Vec<_>>()
    };
    stack_aligned(
        Align::TopLeading,
        modules
            .into_iter()
            .map(|module| {
                let position = module.module.position();
                let target_x = position.x as i16 + preview.dx;
                let target_y = position.y as i16 + preview.dy;
                module_tile(PREVIEW_ID_OFFSET + cell_id(position), &module, 0.58, app).offset(
                    target_x as f32 * (CELL + GAP) + TILE_PAD - view_x,
                    target_y as f32 * (CELL + GAP) + TILE_PAD - view_y,
                )
            })
            .collect(),
    )
    .width(width)
    .height(height)
}

fn modules_in_rect(state: &GuiState, min: GridPos, max: GridPos) -> Vec<ModuleId> {
    state
        .grid_render_modules()
        .into_iter()
        .filter(|module| module_overlaps_rect(module, min, max))
        .map(|module| module.module.id())
        .collect()
}

fn module_overlaps_rect(module: &GridRenderModule<'_>, min: GridPos, max: GridPos) -> bool {
    module.module.position().x <= max.x
        && module.module.position().x + module.width > min.x
        && module.module.position().y <= max.y
        && module.module.position().y + module.height > min.y
}

fn connection_layer<'a>(
    state: &'a GuiState,
    app: &mut PaneState,
    id_offset: u64,
    hidden: &[ModuleId],
    view: GridPos,
    size: GridViewSize,
) -> View<'a, GuiState> {
    let mut layers = Vec::new();
    let modules = state.grid_render_modules();
    for (index, connection) in state.connections().into_iter().enumerate() {
        if hidden.contains(&connection.from()) || hidden.contains(&connection.to()) {
            continue;
        }
        let Some(source) = modules
            .iter()
            .find(|module| module.module.id() == connection.from())
        else {
            continue;
        };
        let Some(target) = modules
            .iter()
            .find(|module| module.module.id() == connection.to())
        else {
            continue;
        };
        let id = id_offset + 60_000 + index as u64 * 10;
        let Some(segment) = connection_segment(source, target, connection) else {
            continue;
        };
        let left = view.x as f32 * (CELL + GAP);
        let top = view.y as f32 * (CELL + GAP);
        let right = left + grid_span(size.columns());
        let bottom = top + grid_span(size.rows());
        if segment.x + segment.width < left
            || segment.x > right
            || segment.y + segment.height < top
            || segment.y > bottom
        {
            continue;
        }
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
        .width(grid_span(state.grid_size().0))
        .height(grid_span(state.grid_size().1))
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WireSegment {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn connection_segment(
    source: &GridRenderModule<'_>,
    target: &GridRenderModule<'_>,
    connection: crate::model::Connection,
) -> Option<WireSegment> {
    if source.module.position().x + source.width <= target.module.position().x {
        if let (Some(source_y), Some(target_y)) = (
            source
                .right_output_offsets
                .get(connection.output())
                .copied()
                .flatten(),
            target
                .left_input_offsets
                .get(connection.input())
                .copied()
                .flatten(),
        ) {
            let source_y = source.module.position().y + source_y;
            let target_y = target.module.position().y + target_y;
            if source_y == target_y {
                return Some(wire_segment(
                    GridPos::new(source.module.position().x + source.width - 1, source_y),
                    Orientation::Right,
                    GridPos::new(target.module.position().x, target_y),
                ));
            }
        }
    }
    if source.module.position().y + source.height <= target.module.position().y {
        if let (Some(source_x), Some(target_x)) = (
            source
                .bottom_output_offsets
                .get(connection.output())
                .copied()
                .flatten(),
            target
                .top_input_offsets
                .get(connection.input())
                .copied()
                .flatten(),
        ) {
            let source_x = source.module.position().x + source_x;
            let target_x = target.module.position().x + target_x;
            if source_x == target_x {
                return Some(wire_segment(
                    GridPos::new(source_x, source.module.position().y + source.height - 1),
                    Orientation::Down,
                    GridPos::new(target_x, target.module.position().y),
                ));
            }
        }
    }
    None
}

fn wire_segment(from: GridPos, orientation: Orientation, to: GridPos) -> WireSegment {
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

fn module_layer<'a>(
    state: &'a GuiState,
    app: &mut PaneState,
    id_offset: u64,
    hidden: &[ModuleId],
    view: GridPos,
    size: GridViewSize,
) -> View<'a, GuiState> {
    let max = GridPos::new(
        view.x.saturating_add(size.columns()),
        view.y.saturating_add(size.rows()),
    );
    stack_aligned(
        Align::TopLeading,
        state
            .grid_render_modules()
            .into_iter()
            .filter(|module| !hidden.contains(&module.module.id()))
            .filter(|module| module_overlaps_rect(module, view, max))
            .map(|module| {
                let position = module.module.position();
                module_tile(id_offset + cell_id(position), &module, 1., app).offset(
                    position.x as f32 * (CELL + GAP) + TILE_PAD,
                    position.y as f32 * (CELL + GAP) + TILE_PAD,
                )
            })
            .collect(),
    )
    .width(grid_span(state.grid_size().0))
    .height(grid_span(state.grid_size().1))
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
    let expand_panel = matches!(state.mode(), Mode::EnvEdit { .. } | Mode::ProbeEdit { .. });
    match (state.mode(), module) {
        (Mode::Edit { parameter, .. }, Some(module)) => {
            rows.push(editor_header(500, module.label(), "Parameters", app));
            for (index, parameter_row_value) in module.parameters().into_iter().enumerate() {
                rows.push(parameter_row(
                    index,
                    parameter == index,
                    parameter_row_value,
                    app,
                ));
            }
            if module.kind() == ModuleKind::Sample {
                let module_id = module.id();
                rows.push(
                    button(3_200, binding!(state.sample_button))
                        .surface(|state, app| {
                            let fill = if state.hovered {
                                Color::from_rgb8(43, 49, 58)
                            } else {
                                field()
                            };
                            rect(3_201)
                                .fill(fill)
                                .stroke(line(), Stroke::new(1.))
                                .corner_rounding(6.)
                                .build(app)
                        })
                        .label(|_, app| {
                            text(3_202, "Choose sample")
                                .font_size(12)
                                .fill(fg())
                                .view()
                                .build(app)
                                .pad_x(9.)
                                .pad_y(5.)
                        })
                        .on_click(move |state, _| state.request_relink_sample(module_id))
                        .build(app)
                        .height(28.),
                );
            }
            if module.kind().has_visual_editor() {
                rows.push(special_editor_button(
                    parameter == module.parameters().len(),
                    app,
                ));
            }
        }
        (Mode::AdsrEdit { parameter, .. }, Some(module)) => {
            rows.push(editor_header(500, module.label(), "Envelope shape", app));
            rows.push(adsr_editor(module, parameter, app));
        }
        (Mode::EnvEdit { point, editing, .. }, Some(module)) => {
            rows.push(editor_header(500, module.label(), "Envelope points", app));
            rows.push(
                envelope_editor(module.id(), module.env_points(), point, editing, app).expand_y(),
            );
        }
        (Mode::ProbeEdit { .. }, Some(module)) => {
            rows.push(editor_header(500, module.label(), "Signal probe", app));
            rows.push(
                probe_editor(state.probe_history(module.id()), state.probe_len(), app).expand_y(),
            );
        }
        (Mode::SampleView { zoom, offset, .. }, Some(module)) => {
            rows.push(editor_header(500, module.label(), "Sample window", app));
            rows.push(sample_editor(module, zoom, offset, app));
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
    let content = column_spaced(5., rows).pad(10.);
    let content = if expand_panel {
        content.expand_y()
    } else {
        content
    };
    let panel = stack(vec![
        rect(501)
            .fill(panel())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(8.)
            .build(app)
            .inert(),
        content,
    ]);
    if expand_panel {
        panel.expand_y()
    } else {
        panel
    }
}

fn editor_header<'a>(
    id: u64,
    module: &'a str,
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
    parameter: crate::model::ModuleParameter,
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
                    PANEL_GAP,
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
        Mode::LoadConfirm => ("Unsaved changes", "Save before opening another patch?"),
        Mode::SaveConfirm => ("Save", "Overwrite or save as a new file?"),
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
    } else if matches!(state.mode(), Mode::LoadConfirm | Mode::SaveConfirm) {
        space()
    } else {
        let value = match state.mode() {
            Mode::QuitConfirm => "Press Enter to quit".to_string(),
            Mode::SaveConfirm => "Press Enter to overwrite".to_string(),
            Mode::ExportConfirm => "Press Enter to overwrite".to_string(),
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
                input.height(
                    if matches!(state.mode(), Mode::LoadConfirm | Mode::SaveConfirm) {
                        0.
                    } else {
                        38.
                    },
                ),
                if matches!(state.mode(), Mode::LoadConfirm) {
                    row_spaced(
                        PANEL_GAP,
                        vec![
                            action_button(
                                binding!(state.cancel_button),
                                "Cancel",
                                GuiAction::Cancel,
                                false,
                                app,
                            )
                            .width(78.),
                            action_button(
                                binding!(state.discard_button),
                                "Don't Save",
                                GuiAction::Delete,
                                false,
                                app,
                            )
                            .width(104.),
                            action_button(
                                binding!(state.confirm_button),
                                "Save",
                                GuiAction::Confirm,
                                true,
                                app,
                            )
                            .width(72.),
                        ],
                    )
                } else if matches!(state.mode(), Mode::SaveConfirm) {
                    row_spaced(
                        PANEL_GAP,
                        vec![
                            action_button(
                                binding!(state.cancel_button),
                                "Cancel",
                                GuiAction::Cancel,
                                false,
                                app,
                            )
                            .width(78.),
                            action_button(
                                binding!(state.discard_button),
                                "Save As",
                                GuiAction::SaveAs,
                                false,
                                app,
                            )
                            .width(88.),
                            action_button(
                                binding!(state.confirm_button),
                                "Overwrite",
                                GuiAction::Confirm,
                                true,
                                app,
                            )
                            .width(92.),
                        ],
                    )
                } else {
                    row_spaced(
                        PANEL_GAP,
                        vec![
                            action_button(
                                binding!(state.cancel_button),
                                "Cancel",
                                GuiAction::Cancel,
                                false,
                                app,
                            )
                            .width(82.),
                            action_button(
                                binding!(state.confirm_button),
                                "Confirm",
                                GuiAction::Confirm,
                                true,
                                app,
                            )
                            .width(92.),
                        ],
                    )
                }
                .height(28.),
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
    module: ModuleId,
    points: &'a [crate::model::EnvPoint],
    selected: usize,
    editing: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let graph_points = points
        .iter()
        .map(|point| {
            brainwash::patch::EnvPoint::new(
                brainwash::sample::Unit::new(envelope_point_time(*point)).unwrap(),
                brainwash::sample::Sample::new(envelope_point_value(*point)).unwrap(),
                point.curve,
            )
        })
        .collect::<Vec<_>>();
    let point_views = points.to_vec();
    let value_labels = row_spaced(
        PANEL_GAP,
        points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                let marker = if point.curve { "~" } else { "/" };
                text(
                    880 + index as u64,
                    format!(
                        "{marker}{:.2},{:+.2}",
                        envelope_point_time(*point),
                        envelope_point_value(*point)
                    ),
                )
                .font_size(12)
                .fill(if selected == index {
                    if editing { fg() } else { accent() }
                } else {
                    quiet()
                })
                .view()
                .build(app)
            })
            .collect(),
    );
    let graph = draw(move |area, app: &mut PaneState| {
        let graph_points_for_path = graph_points.clone();
        let width = (area.width - 20.).max(1.);
        let height = (area.height - 20.).max(1.);
        let mut graph_layers = vec![
            rect(842)
                .fill(Color::TRANSPARENT)
                .stroke(quiet(), Stroke::new(1.))
                .corner_rounding(3.)
                .build(app)
                .pad(10.)
                .inert(),
            path(841, move |area| envelope_path(area, &graph_points_for_path))
                .stroke(Color::from_rgb8(255, 200, 100), Stroke::new(2.))
                .build(app)
                .pad(10.),
        ];
        graph_layers.extend(point_views.iter().enumerate().map(|(index, point)| {
            let x = envelope_point_time(*point).clamp(0., 1.);
            let y = envelope_point_y(envelope_point_value(*point));
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
        graph_layers.push(
            rect(843)
                .fill(Color::TRANSPARENT)
                .view()
                .gesture(gesture::drag(844).button(MouseButton::Left).run(
                    move |state: &mut GuiState, _app, drag| {
                        let (phase, point) = match drag {
                            DragPhase::Began { start, .. } => (EnvPointerPhase::Start, start),
                            DragPhase::Updated { current, .. } => (EnvPointerPhase::Drag, current),
                            DragPhase::Completed { current, .. } => (EnvPointerPhase::End, current),
                        };
                        let time = (point.x / width).clamp(0., 1.);
                        let y = (point.y / height).clamp(0., 1.);
                        let value = (ENVELOPE_VALUE_MAX
                            - y * (ENVELOPE_VALUE_MAX - ENVELOPE_VALUE_MIN))
                            .clamp(ENVELOPE_VALUE_MIN, ENVELOPE_VALUE_MAX);
                        state.drag_env_point(phase, module, time, value);
                    },
                ))
                .build(app)
                .pad(10.),
        );
        stack(graph_layers).draw(area, app)
    });
    stack(vec![
        rect(840)
            .fill(field())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(6.)
            .build(app)
            .inert(),
        column_spaced(8., vec![value_labels, graph.expand_y()]).pad(10.),
    ])
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProbeStats {
    current: f32,
    min: f32,
    max: f32,
}

fn probe_stats(history: &[f32]) -> ProbeStats {
    if history.is_empty() {
        return ProbeStats {
            current: 0.,
            min: 0.,
            max: 0.,
        };
    }
    ProbeStats {
        current: history.last().copied().unwrap_or(0.),
        min: history.iter().copied().fold(f32::INFINITY, f32::min),
        max: history.iter().copied().fold(f32::NEG_INFINITY, f32::max),
    }
}

fn probe_value_text<'a>(
    id: u64,
    label: &'static str,
    value: f32,
    color: Color,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    text(id, format!("{label} {value:+.4}"))
        .font_size(12)
        .fill(color)
        .view()
        .build(app)
}

fn probe_editor<'a>(history: Vec<f32>, len: u32, app: &mut PaneState) -> View<'a, GuiState> {
    let stats = probe_stats(&history);
    let graph = history;
    stack(vec![
        rect(900)
            .fill(field())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(6.)
            .build(app),
        column_spaced(
            8.,
            vec![
                row_spaced(
                    PANEL_GAP,
                    vec![
                        probe_value_text(903, "Value", stats.current, fg(), app),
                        probe_value_text(904, "Min", stats.min, quiet(), app),
                        probe_value_text(905, "Max", stats.max, quiet(), app),
                        text(906, format!("{} samples", len))
                            .font_size(12)
                            .fill(quiet())
                            .view()
                            .build(app),
                    ],
                ),
                stack(vec![
                    rect(901)
                        .fill(Color::TRANSPARENT)
                        .stroke(quiet(), Stroke::new(1.))
                        .corner_rounding(3.)
                        .build(app)
                        .inert(),
                    path(902, move |area| probe_path(area, &graph))
                        .stroke(Color::from_rgb8(96, 210, 210), Stroke::new(2.))
                        .build(app)
                        .pad(10.),
                ])
                .expand_y(),
            ],
        )
        .pad(10.),
    ])
}

fn sample_editor<'a>(
    module: &Module,
    zoom: u16,
    offset: u16,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let (waveform, start, visible) = module.sample_waveform(zoom, offset);
    stack(vec![
        rect(920)
            .fill(field())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(6.)
            .build(app),
        path(921, move |area| sample_path(area, &waveform))
            .stroke(Color::from_rgb8(96, 160, 235), Stroke::new(1.5))
            .build(app)
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

fn sample_path(area: Area, samples: &[f32]) -> BezPath {
    let mut path = BezPath::new();
    if samples.is_empty() {
        let y = area.y + area.height * 0.5;
        path.move_to((area.x as f64, y as f64));
        path.line_to(((area.x + area.width) as f64, y as f64));
        return path;
    }
    for (index, sample) in samples.iter().copied().enumerate() {
        let x = area.x + index as f32 / (samples.len() - 1).max(1) as f32 * area.width;
        let y = area.y + (1.0 - (sample.clamp(-1.0, 1.0) + 1.0) * 0.5) * area.height;
        if index == 0 {
            path.move_to((x as f64, y as f64));
        } else {
            path.line_to((x as f64, y as f64));
        }
    }
    path
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

fn envelope_path(area: Area, points: &[brainwash::patch::EnvPoint]) -> BezPath {
    let mut path = BezPath::new();
    if points.is_empty() {
        return path;
    }
    let width = area.width.max(1.);
    let height = area.height.max(1.);
    for index in 0..=48 {
        let time = index as f32 / 48.;
        let value = brainwash::patch::envelope_value(points, time);
        let x = area.x + time * width;
        let y = area.y + envelope_point_y(value) * height;
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

fn envelope_point_time(point: crate::model::EnvPoint) -> f32 {
    point.time as f32 / 100.
}

fn envelope_point_value(point: crate::model::EnvPoint) -> f32 {
    point.value as f32 / 100.
}

fn envelope_point_y(value: f32) -> f32 {
    let range = ENVELOPE_VALUE_MAX - ENVELOPE_VALUE_MIN;
    (1. - ((value - ENVELOPE_VALUE_MIN) / range).clamp(0., 1.)).clamp(0., 1.)
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
            Color::from_rgb8(255, 96, 96)
        } else {
            accent()
        }
    } else if curved {
        Color::from_rgb8(255, 200, 100)
    } else {
        Color::from_rgb8(234, 238, 242)
    };
    let stroke = if selected && editing {
        Color::from_rgb8(255, 200, 100)
    } else if selected {
        fg()
    } else {
        field()
    };
    let stroke_width = if selected && editing {
        2.5
    } else if selected {
        2.
    } else {
        1.
    };
    path(id, move |area| {
        envelope_point_path(area, x, y, curved, selected)
    })
    .fill(fill)
    .stroke(stroke, Stroke::new(stroke_width))
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
        ParameterValue::Time { value, .. } => (*value as f32 / 10_000.).clamp(0., 1.),
        ParameterValue::Bars {
            numerator,
            denominator,
        } => (*numerator as f32 / (*denominator).max(1) as f32 / 16.).clamp(0., 1.),
        ParameterValue::File { .. } => 1.,
        ParameterValue::Enum { index, options } => {
            if options.len() <= 1 {
                1.
            } else {
                *index as f32 / (options.len() - 1) as f32
            }
        }
        ParameterValue::Text(_) => 1.,
    }
}

fn grid_cell<'a>(
    state: &'a GuiState,
    position: GridPos,
    id_offset: u64,
    show_selection: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = id_offset + cell_id(position);
    let cursor = state.cursor() == position;
    let selected = show_selection
        && state.selection().is_some_and(|(anchor, extent)| {
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
            } else if cursor {
                Color::from_rgb8(42, 46, 54)
            } else {
                Color::from_rgb8(26, 29, 34)
            })
            .stroke(
                if cursor {
                    Color::from_rgb8(248, 250, 252)
                } else {
                    line()
                },
                Stroke::new(if cursor { 3. } else { 1. }),
            )
            .corner_rounding(6.)
            .build(app),
    ];

    stack(layers).height(CELL).width(CELL)
}

fn module_tile<'a>(
    id: u64,
    module: &GridRenderModule<'a>,
    alpha: f32,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let alpha = alpha.clamp(0., 1.);
    let kind = module.module.kind();
    let mut code = module.module.label().chars().take(3).collect::<String>();
    code.make_ascii_uppercase();
    let input_count = module.input_count;
    let output_count = module.output_count;
    let tile_width = module_span(module.width);
    let tile_height = module_span(module.height);
    let meter_inset = (METER_SIZE - PORT_SIZE) * 0.5;
    let mut layers = vec![
        rect(id + 1)
            .fill(
                if module.module.disabled() {
                    Color::from_rgb8(160, 42, 54)
                } else {
                    module_color(kind.category())
                }
                .with_alpha(alpha),
            )
            .stroke(
                if module.module.disabled() {
                    Color::from_rgb8(248, 92, 92)
                } else {
                    Color::from_rgb8(10, 12, 14)
                }
                .with_alpha(alpha),
                Stroke::new(if module.module.disabled() { 2. } else { 1. }),
            )
            .corner_rounding(6.)
            .build(app),
    ];
    if kind != ModuleKind::Probe {
        layers.push(
            rect(id + 9)
                .fill(Color::from_rgb8(8, 10, 12).with_alpha(0.42 * alpha))
                .corner_rounding(5.)
                .build(app)
                .height(10.)
                .width(tile_width),
        );
        layers.push(
            text(id + 2, code)
                .font_size(8)
                .fill(Color::from_rgb8(248, 250, 252).with_alpha(alpha))
                .view()
                .build(app)
                .width(tile_width)
                .height(10.),
        );
    }

    match kind {
        ModuleKind::TurnRightDown => {
            layers.push(
                input_port(
                    id + 30_000,
                    module.input_connected[0],
                    module.meter_values.first().copied(),
                    alpha,
                    app,
                )
                .offset(PORT_INSET - meter_inset, port_axis(0) - meter_inset),
            );
            layers.push(
                output_port(id + 33_000, alpha, app)
                    .offset(port_axis(0), tile_height - PORT_INSET - PORT_SIZE),
            );
        }
        ModuleKind::TurnDownRight => {
            layers.push(
                input_port(
                    id + 31_000,
                    module.input_connected[0],
                    module.meter_values.first().copied(),
                    alpha,
                    app,
                )
                .offset(port_axis(0) - meter_inset, PORT_INSET - meter_inset),
            );
            layers.push(
                output_port(id + 32_000, alpha, app)
                    .offset(tile_width - PORT_INSET - PORT_SIZE, port_axis(0)),
            );
        }
        ModuleKind::LeftSplit => {
            layers.push(
                input_port(
                    id + 30_000,
                    module.input_connected[0],
                    module.meter_values.first().copied(),
                    alpha,
                    app,
                )
                .offset(PORT_INSET - meter_inset, port_axis(0) - meter_inset),
            );
            layers.push(
                output_port(id + 33_000, alpha, app)
                    .offset(port_axis(0), tile_height - PORT_INSET - PORT_SIZE),
            );
            layers.push(
                output_port(id + 32_001, alpha, app)
                    .offset(tile_width - PORT_INSET - PORT_SIZE, port_axis(0)),
            );
        }
        ModuleKind::TopSplit => {
            layers.push(
                input_port(
                    id + 31_000,
                    module.input_connected[0],
                    module.meter_values.first().copied(),
                    alpha,
                    app,
                )
                .offset(port_axis(0) - meter_inset, PORT_INSET - meter_inset),
            );
            layers.push(
                output_port(id + 33_000, alpha, app)
                    .offset(port_axis(0), tile_height - PORT_INSET - PORT_SIZE),
            );
            layers.push(
                output_port(id + 32_001, alpha, app)
                    .offset(tile_width - PORT_INSET - PORT_SIZE, port_axis(0)),
            );
        }
        ModuleKind::RightJoin => {
            layers.push(
                input_port(
                    id + 30_000,
                    module.input_connected[0],
                    module.meter_values.first().copied(),
                    alpha,
                    app,
                )
                .offset(PORT_INSET - meter_inset, port_axis(0) - meter_inset),
            );
            layers.push(
                input_port(
                    id + 31_001,
                    module.input_connected[1],
                    module.meter_values.get(1).copied(),
                    alpha,
                    app,
                )
                .offset(port_axis(0) - meter_inset, PORT_INSET - meter_inset),
            );
            layers.push(
                output_port(id + 32_000, alpha, app)
                    .offset(tile_width - PORT_INSET - PORT_SIZE, port_axis(0)),
            );
        }
        ModuleKind::DownJoin => {
            layers.push(
                input_port(
                    id + 30_000,
                    module.input_connected[0],
                    module.meter_values.first().copied(),
                    alpha,
                    app,
                )
                .offset(PORT_INSET - meter_inset, port_axis(0) - meter_inset),
            );
            layers.push(
                input_port(
                    id + 31_001,
                    module.input_connected[1],
                    module.meter_values.get(1).copied(),
                    alpha,
                    app,
                )
                .offset(port_axis(0) - meter_inset, PORT_INSET - meter_inset),
            );
            layers.push(
                output_port(id + 33_000, alpha, app)
                    .offset(port_axis(0), tile_height - PORT_INSET - PORT_SIZE),
            );
        }
        _ => {
            if module.has_input_left {
                for index in 0..input_count {
                    let y = port_axis(index);
                    let port_id = if index == 0 && output_count == 0 {
                        id + 3
                    } else {
                        id + 30_000 + index as u64
                    };
                    layers.push(
                        input_port(
                            port_id,
                            module.input_connected[index as usize],
                            module.meter_values.get(index as usize).copied(),
                            alpha,
                            app,
                        )
                        .offset(PORT_INSET - meter_inset, y - meter_inset),
                    );
                }
            }

            if module.has_input_top {
                for index in 0..input_count {
                    let x = port_axis(index);
                    layers.push(
                        input_port(
                            id + 31_000 + index as u64,
                            module.input_connected[index as usize],
                            module.meter_values.get(index as usize).copied(),
                            alpha,
                            app,
                        )
                        .offset(x - meter_inset, PORT_INSET - meter_inset),
                    );
                }
            }

            if module.has_output_right {
                for index in 0..output_count {
                    let Some(offset) = module.right_output_offsets[index as usize] else {
                        continue;
                    };
                    let port_id = if output_count == 1 {
                        id + 4
                    } else {
                        id + 32_000 + index as u64
                    };
                    layers.push(
                        output_port(port_id, alpha, app)
                            .offset(tile_width - PORT_INSET - PORT_SIZE, port_axis(offset)),
                    );
                }
            }

            if module.has_output_bottom {
                for index in 0..output_count {
                    let Some(offset) = module.bottom_output_offsets[index as usize] else {
                        continue;
                    };
                    layers.push(
                        output_port(id + 33_000 + index as u64, alpha, app)
                            .offset(port_axis(offset), tile_height - PORT_INSET - PORT_SIZE),
                    );
                }
            }
        }
    }

    if matches!(kind, ModuleKind::RightJoin | ModuleKind::DownJoin) {
        layers.push(port_label(
            id + 36_000,
            format!("{}{}", module.input_labels[0], module.input_labels[1]),
            alpha,
            app,
        ));
    } else if kind != ModuleKind::Probe {
        if module.has_input_left {
            for index in 0..input_count {
                layers.push(
                    port_label(
                        id + 36_000 + index as u64,
                        module.input_labels[index as usize].to_string(),
                        alpha,
                        app,
                    )
                    .offset(0., index as f32 * (CELL + GAP)),
                );
            }
        }
        if module.has_input_top {
            for index in 0..input_count {
                layers.push(
                    port_label(
                        id + 37_000 + index as u64,
                        module.input_labels[index as usize].to_string(),
                        alpha,
                        app,
                    )
                    .offset(index as f32 * (CELL + GAP), 0.),
                );
            }
        }
    }

    if let Some(value) = module.probe_value {
        let track_width = (tile_width - 8.).max(1.);
        let level = value.abs().clamp(0., 1.);
        layers.push(
            rect(id + 6)
                .fill(Color::from_rgb8(8, 10, 12).with_alpha(0.62 * alpha))
                .corner_rounding(2.)
                .build(app)
                .height(2.)
                .width(track_width)
                .offset(4., tile_height - 4.),
        );
        layers.push(
            rect(id + 7)
                .fill(Color::from_rgb8(96, 210, 210).with_alpha(alpha))
                .corner_rounding(2.)
                .build(app)
                .height(2.)
                .width((track_width * level).max(1.))
                .offset(4., tile_height - 4.),
        );
        layers.push(
            text(id + 8, format!("{value:+.2}"))
                .font_size(7)
                .fill(Color::from_rgb8(248, 250, 252).with_alpha(alpha))
                .view()
                .build(app)
                .width(tile_width)
                .height(8.)
                .offset(0., 1.),
        );
    }

    stack_aligned(Align::TopLeading, layers)
        .width(tile_width)
        .height(tile_height)
}

fn input_port<'a>(
    id: u64,
    connected: bool,
    meter_value: Option<f32>,
    alpha: f32,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let has_meter = meter_value.is_some();
    let port = if connected {
        circle(id)
            .fill(if has_meter {
                Color::TRANSPARENT
            } else {
                Color::from_rgb8(14, 16, 20).with_alpha(alpha)
            })
            .stroke(
                Color::from_rgb8(248, 250, 252).with_alpha(alpha),
                Stroke::new(1.),
            )
            .view()
            .build(app)
            .width(PORT_SIZE)
            .height(PORT_SIZE)
    } else {
        svg(id + 5_000, include_str!("../assets/port-disconnected.svg"))
            .fill(Color::from_rgb8(248, 250, 252).with_alpha(alpha))
            .view()
            .build(app)
            .width(PORT_SIZE)
            .height(PORT_SIZE)
    };
    let mut layers = Vec::new();
    if let Some(value) = meter_value {
        layers.push(meter_indicator(id + 10_000, value, alpha, app));
    }
    layers.push(port);
    stack_aligned(Align::CenterCenter, layers)
        .width(METER_SIZE)
        .height(METER_SIZE)
}

fn output_port<'a>(id: u64, alpha: f32, app: &mut PaneState) -> View<'a, GuiState> {
    circle(id)
        .fill(Color::from_rgb8(248, 250, 252).with_alpha(alpha))
        .view()
        .build(app)
        .width(PORT_SIZE)
        .height(PORT_SIZE)
}

fn meter_indicator<'a>(id: u64, value: f32, alpha: f32, app: &mut PaneState) -> View<'a, GuiState> {
    let level = value.abs().clamp(0., 1.);
    stack_aligned(
        Align::CenterCenter,
        vec![
            circle(id)
                .stroke(Color::BLACK.with_alpha(alpha), Stroke::new(2.))
                .view()
                .build(app)
                .width(METER_SIZE)
                .height(METER_SIZE),
            path(id + 1, move |area| meter_arc_path(area, level))
                .stroke(
                    Color::from_rgb8(96, 210, 210).with_alpha(alpha),
                    Stroke::new(2.).with_caps(Cap::Round),
                )
                .build(app)
                .width(METER_SIZE)
                .height(METER_SIZE),
        ],
    )
    .width(METER_SIZE)
    .height(METER_SIZE)
}

fn meter_arc_path(area: Area, level: f32) -> BezPath {
    let mut path = BezPath::new();
    if level <= 0. {
        return path;
    }
    let center_x = area.x + area.width * 0.5;
    let center_y = area.y + area.height * 0.5;
    let radius = (area.width.min(area.height) * 0.5 - 1.).max(0.);
    let sweep = level * std::f32::consts::TAU;
    let segments = (level * 32.).ceil().max(1.) as u16;
    for index in 0..=segments {
        let angle = -std::f32::consts::FRAC_PI_2 + sweep * index as f32 / segments as f32;
        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();
        if index == 0 {
            path.move_to((x as f64, y as f64));
        } else {
            path.line_to((x as f64, y as f64));
        }
    }
    path
}

fn port_label<'a>(id: u64, label: String, alpha: f32, app: &mut PaneState) -> View<'a, GuiState> {
    stack_aligned(
        Align::CenterCenter,
        vec![
            text(id, label)
                .font_size(9)
                .fill(Color::from_rgb8(248, 250, 252).with_alpha(0.8 * alpha))
                .view()
                .build(app),
        ],
    )
    .width(CELL - TILE_PAD * 2.)
    .height(CELL - TILE_PAD * 2.)
}

fn module_span(cells: u16) -> f32 {
    cells as f32 * CELL + cells.saturating_sub(1) as f32 * GAP - TILE_PAD * 2.
}

fn port_axis(index: u16) -> f32 {
    index as f32 * (CELL + GAP) + CELL * 0.5 - TILE_PAD - PORT_SIZE * 0.5
}

fn grid_pointer_surface(
    app: &mut PaneState,
    view: GridPos,
    size: GridViewSize,
    area: Area,
) -> View<'static, GuiState> {
    rect(990)
        .fill(Color::TRANSPARENT)
        .view()
        .gesture(gesture::click(991).button(MouseButton::Left).run(
            move |state: &mut GuiState, _app, event| {
                if matches!(event.state, ClickPhase::Completed)
                    && let Some(position) = grid_position(event.location.local(), view, size, area)
                {
                    state.set_grid_view_size(size);
                    state.click_grid_cell(position);
                }
            },
        ))
        .gesture(gesture::drag(992).button(MouseButton::Left).run(
            move |state: &mut GuiState, _app, drag| match drag {
                DragPhase::Began { start, .. } => {
                    if let Some(position) = grid_position(start, view, size, area) {
                        state.set_grid_view_size(size);
                        state.drag_grid_cell(GridPointerPhase::Start, position);
                    }
                }
                DragPhase::Updated { current, .. } => {
                    if let Some(position) = grid_position(current, view, size, area) {
                        state.set_grid_view_size(size);
                        state.drag_grid_cell(GridPointerPhase::Drag, position);
                    }
                }
                DragPhase::Completed { current, .. } => {
                    state.set_grid_view_size(size);
                    let position =
                        grid_position(current, view, size, area).unwrap_or_else(|| state.cursor());
                    state.drag_grid_cell(GridPointerPhase::End, position);
                }
            },
        ))
        .build(app)
}

fn grid_position(point: Point, view: GridPos, size: GridViewSize, area: Area) -> Option<GridPos> {
    let (inset_x, inset_y) = grid_inset(area, size);
    let point = Point::new(point.x - inset_x, point.y - inset_y);
    let width = grid_span(size.columns());
    let height = grid_span(size.rows());
    if point.x < 0. || point.y < 0. || point.x >= width || point.y >= height {
        return None;
    }
    let x = (point.x / (CELL + GAP)).floor() as u16;
    let y = (point.y / (CELL + GAP)).floor() as u16;
    let cell_x = point.x - x as f32 * (CELL + GAP);
    let cell_y = point.y - y as f32 * (CELL + GAP);
    (x < size.columns() && y < size.rows() && cell_x < CELL && cell_y < CELL)
        .then_some(GridPos::new(x + view.x, y + view.y))
}

fn grid_view_size(state: &GuiState, area: Area) -> GridViewSize {
    let (columns, rows) = state.grid_size();
    GridViewSize::new(
        visible_grid_cells(area.width).min(columns),
        visible_grid_cells(area.height).min(rows),
    )
}

fn grid_inset(area: Area, size: GridViewSize) -> (f32, f32) {
    (
        ((area.width - grid_span(size.columns())) * 0.5).max(0.),
        ((area.height - grid_span(size.rows())) * 0.5).max(0.),
    )
}

fn visible_grid_cells(length: f32) -> u16 {
    ((length.max(0.) + GAP) / (CELL + GAP)).floor().max(1.) as u16
}

fn grid_span(cells: u16) -> f32 {
    cells as f32 * CELL + cells.saturating_sub(1) as f32 * GAP
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
    module: &PaletteModule,
    selected: bool,
    width: f32,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = 2_000 + index as u64;
    let color = module_color(module.category());
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
            .width(width)
            .height(PALETTE_ROW_HEIGHT),
        palette_module_label(id + 200, module, app),
    ])
}

fn filtered_module_choice<'a>(
    index: usize,
    module: &PaletteModule,
    selected: bool,
    width: f32,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = 2_500 + index as u64;
    let color = module_color(module.category());
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
            .width(width)
            .height(PALETTE_ROW_HEIGHT),
        palette_module_label(id + 200, module, app),
    ])
}

fn palette_module_label<'a>(
    id: u64,
    module: &PaletteModule,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let label = text(id, module.label())
        .font_size(13)
        .fill(fg())
        .view()
        .build(app);
    if module.category() != ModuleCategory::Routing {
        return label.pad_x(10.).pad_y(5.);
    }
    let kind = module.kind();
    row_spaced_aligned(
        7.,
        Align::CenterLeading,
        vec![
            path(id + 10_000, move |area| routing_icon_path(area, kind))
                .stroke(fg(), Stroke::new(1.6))
                .build(app)
                .width(PALETTE_ICON_SIZE)
                .height(PALETTE_ICON_SIZE),
            label,
        ],
    )
    .pad_x(8.)
    .pad_y(3.)
}

fn routing_icon_path(area: Area, kind: ModuleKind) -> BezPath {
    let point = |x: f32, y: f32| {
        (
            (area.x + area.width * x / 16.) as f64,
            (area.y + area.height * y / 16.) as f64,
        )
    };
    let mut path = BezPath::new();
    match kind {
        ModuleKind::TurnRightDown => {
            path.move_to(point(1., 4.));
            path.line_to(point(11., 4.));
            path.line_to(point(11., 15.));
            routing_arrow_down(&mut path, area, 11., 15.);
        }
        ModuleKind::TurnDownRight => {
            path.move_to(point(4., 1.));
            path.line_to(point(4., 11.));
            path.line_to(point(15., 11.));
            routing_arrow_right(&mut path, area, 15., 11.);
        }
        ModuleKind::LeftSplit => {
            path.move_to(point(1., 5.));
            path.line_to(point(9., 5.));
            path.line_to(point(15., 5.));
            path.move_to(point(9., 5.));
            path.line_to(point(9., 15.));
            routing_arrow_right(&mut path, area, 15., 5.);
            routing_arrow_down(&mut path, area, 9., 15.);
        }
        ModuleKind::TopSplit => {
            path.move_to(point(5., 1.));
            path.line_to(point(5., 9.));
            path.line_to(point(5., 15.));
            path.move_to(point(5., 9.));
            path.line_to(point(15., 9.));
            routing_arrow_right(&mut path, area, 15., 9.);
            routing_arrow_down(&mut path, area, 5., 15.);
        }
        ModuleKind::RightJoin => {
            path.move_to(point(1., 9.));
            path.line_to(point(9., 9.));
            path.move_to(point(9., 1.));
            path.line_to(point(9., 9.));
            path.line_to(point(15., 9.));
            routing_arrow_right(&mut path, area, 15., 9.);
        }
        ModuleKind::DownJoin => {
            path.move_to(point(1., 9.));
            path.line_to(point(9., 9.));
            path.move_to(point(9., 1.));
            path.line_to(point(9., 15.));
            routing_arrow_down(&mut path, area, 9., 15.);
        }
        _ => {}
    }
    path
}

fn routing_arrow_right(path: &mut BezPath, area: Area, x: f32, y: f32) {
    let point = |x: f32, y: f32| {
        (
            (area.x + area.width * x / 16.) as f64,
            (area.y + area.height * y / 16.) as f64,
        )
    };
    path.move_to(point(x - 3., y - 3.));
    path.line_to(point(x, y));
    path.line_to(point(x - 3., y + 3.));
}

fn routing_arrow_down(path: &mut BezPath, area: Area, x: f32, y: f32) {
    let point = |x: f32, y: f32| {
        (
            (area.x + area.width * x / 16.) as f64,
            (area.y + area.height * y / 16.) as f64,
        )
    };
    path.move_to(point(x - 3., y - 3.));
    path.line_to(point(x, y));
    path.line_to(point(x + 3., y - 3.));
}

fn action_button<'a>(
    button_state: (&'a ButtonState, haven::Binding<GuiState, ButtonState>),
    label: &'static str,
    action: GuiAction,
    active: bool,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    let id = 3_000 + action_id(action);
    button(id, button_state)
        .surface(move |state, app| {
            let fill = match (active, state.depressed, state.hovered) {
                (_, true, _) => accent().with_alpha(0.72),
                (true, false, true) => accent().with_alpha(0.88),
                (true, false, false) => accent(),
                (false, false, true) => Color::from_rgb8(43, 49, 58),
                (false, false, false) => field(),
            };
            let stroke = if active || state.hovered {
                accent()
            } else {
                line()
            };
            rect(id + 1)
                .fill(fill)
                .stroke(stroke, Stroke::new(1.))
                .corner_rounding(7.)
                .build(app)
        })
        .label(move |state, app| {
            text(id + 2, label)
                .font_size(13)
                .fill(if active && !state.depressed {
                    Color::BLACK
                } else {
                    fg()
                })
                .view()
                .build(app)
                .pad_x(9.)
                .pad_y(6.)
        })
        .on_click(move |state, _| state.apply(action))
        .build(app)
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
            .pad_x(9.)
            .pad_y(6.),
    ])
    .height(28.)
}

fn shortcuts(state: &GuiState) -> Vec<(u8, &'static str, &'static str)> {
    let mut hints = Vec::new();
    for group in 0..=4 {
        for binding in bindings(state) {
            if let Some(hint) = binding.hint {
                let candidate = (binding_group(binding), hint.key, hint.label);
                if candidate.0 == group && !hints.contains(&candidate) {
                    hints.push(candidate);
                }
            }
        }
    }
    hints
}

fn shortcut_panel<'a>(state: &GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let hints = shortcuts(state);
    let mut rows = Vec::new();
    let mut previous_group = None;
    for (index, (group, key, label)) in hints.iter().copied().enumerate() {
        if previous_group.is_some_and(|previous| previous != group) {
            rows.push(space().height(8.));
        }
        let id = 70_000 + index as u64 * 10;
        rows.push(row_spaced(
            6.,
            vec![
                text(id + 1, key)
                    .font_size(11)
                    .fill(accent())
                    .align(Alignment::Start)
                    .view()
                    .build(app),
                rect(id + 3)
                    .fill(quiet().with_alpha(0.35))
                    .build(app)
                    .height(1.)
                    .expand_x(),
                text(id + 2, label)
                    .font_size(11)
                    .fill(fg())
                    .align(Alignment::End)
                    .view()
                    .build(app),
            ],
        ));
        previous_group = Some(group);
    }
    stack(vec![
        rect(69_900)
            .fill(panel())
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(8.)
            .build(app)
            .inert(),
        column_spaced_aligned(3., Align::TopLeading, rows).pad(10.),
    ])
    .width(220.)
}

#[derive(Clone, Copy, PartialEq)]
enum BindingInput {
    Character(char),
    Named(NamedKey),
    Text,
}

#[derive(Clone, Copy)]
enum BindingEffect {
    Action(GuiAction),
    Text,
}

#[derive(Clone, Copy, PartialEq)]
struct BindingHint {
    key: &'static str,
    label: &'static str,
}

#[derive(Clone, Copy)]
struct KeyBinding {
    input: BindingInput,
    effect: BindingEffect,
    hint: Option<BindingHint>,
}

impl KeyBinding {
    const fn action(
        input: BindingInput,
        action: GuiAction,
        key: &'static str,
        label: &'static str,
    ) -> Self {
        Self {
            input,
            effect: BindingEffect::Action(action),
            hint: Some(BindingHint { key, label }),
        }
    }

    const fn text(key: &'static str, label: &'static str) -> Self {
        Self {
            input: BindingInput::Text,
            effect: BindingEffect::Text,
            hint: Some(BindingHint { key, label }),
        }
    }
}

fn binding_group(binding: &KeyBinding) -> u8 {
    match binding.effect {
        BindingEffect::Text => 0,
        BindingEffect::Action(
            GuiAction::Left
            | GuiAction::Down
            | GuiAction::Up
            | GuiAction::Right
            | GuiAction::LeftFast
            | GuiAction::DownFast
            | GuiAction::UpFast
            | GuiAction::RightFast
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TextStart
            | GuiAction::TextEnd,
        ) => 0,
        BindingEffect::Action(
            GuiAction::Confirm
            | GuiAction::Cancel
            | GuiAction::OpenPalette
            | GuiAction::Palette(_)
            | GuiAction::Search
            | GuiAction::Edit
            | GuiAction::Move
            | GuiAction::Copy
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Delete
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::InputChar(_),
        ) => 1,
        BindingEffect::Action(
            GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Instrument(_),
        ) => 2,
        BindingEffect::Action(
            GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::OpenModules
            | GuiAction::SaveModule
            | GuiAction::Quit,
        ) => 3,
        BindingEffect::Action(
            GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::EditComposition
            | GuiAction::ExitComposition,
        ) => 4,
    }
}

const TEXT_INPUT_CHARS: &str = r#"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -_./+#!"$%&'()*,:;<=>?@[\]^`{}|~"#;

const NORMAL_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('H'),
        GuiAction::LeftFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('J'),
        GuiAction::DownFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('K'),
        GuiAction::UpFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('L'),
        GuiAction::RightFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('n'),
        GuiAction::OpenPalette,
        "n",
        "module",
    ),
    KeyBinding::action(
        BindingInput::Character('!'),
        GuiAction::Palette(ModuleCategory::Source),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('@'),
        GuiAction::Palette(ModuleCategory::Shape),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('#'),
        GuiAction::Palette(ModuleCategory::Filter),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('$'),
        GuiAction::Palette(ModuleCategory::Effect),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('%'),
        GuiAction::Palette(ModuleCategory::Logic),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('^'),
        GuiAction::Palette(ModuleCategory::Routing),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('&'),
        GuiAction::Palette(ModuleCategory::Composition),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('*'),
        GuiAction::Palette(ModuleCategory::Output),
        "! ... *",
        "category",
    ),
    KeyBinding::action(BindingInput::Character('i'), GuiAction::Edit, "i", "edit"),
    KeyBinding::action(BindingInput::Character('m'), GuiAction::Move, "m", "move"),
    KeyBinding::action(BindingInput::Character('y'), GuiAction::Copy, "y", "copy"),
    KeyBinding::action(
        BindingInput::Character('o'),
        GuiAction::Rotate,
        "o",
        "rotate",
    ),
    KeyBinding::action(
        BindingInput::Character('r'),
        GuiAction::Delete,
        "r/.",
        "delete",
    ),
    KeyBinding::action(
        BindingInput::Character('.'),
        GuiAction::Delete,
        "r/.",
        "delete",
    ),
    KeyBinding::action(
        BindingInput::Character(','),
        GuiAction::Select,
        ",",
        "select",
    ),
    KeyBinding::action(
        BindingInput::Character('p'),
        GuiAction::EditComposition,
        "p",
        "composition",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::TogglePlay,
        "space",
        "play",
    ),
    KeyBinding::action(
        BindingInput::Character('v'),
        GuiAction::ToggleMeters,
        "v",
        "meters",
    ),
    KeyBinding::action(BindingInput::Character('O'), GuiAction::Load, "O", "open"),
    KeyBinding::action(BindingInput::Character('w'), GuiAction::Save, "w/W", "save"),
    KeyBinding::action(
        BindingInput::Character('W'),
        GuiAction::SaveAs,
        "w/W",
        "save",
    ),
    KeyBinding::action(
        BindingInput::Character('e'),
        GuiAction::Export,
        "e",
        "export",
    ),
    KeyBinding::action(BindingInput::Character('Q'), GuiAction::Quit, "Q", "quit"),
    KeyBinding::action(
        BindingInput::Character('u'),
        GuiAction::Undo,
        "u/U",
        "undo/redo",
    ),
    KeyBinding::action(
        BindingInput::Character('U'),
        GuiAction::Redo,
        "u/U",
        "undo/redo",
    ),
    KeyBinding::action(
        BindingInput::Character('t'),
        GuiAction::TrackEdit,
        "t",
        "track",
    ),
    KeyBinding::action(
        BindingInput::Character('s'),
        GuiAction::TrackSettings,
        "s",
        "settings",
    ),
    KeyBinding::action(
        BindingInput::Character('1'),
        GuiAction::Instrument(0),
        "1-5",
        "instrument",
    ),
    KeyBinding::action(
        BindingInput::Character('2'),
        GuiAction::Instrument(1),
        "1-5",
        "instrument",
    ),
    KeyBinding::action(
        BindingInput::Character('3'),
        GuiAction::Instrument(2),
        "1-5",
        "instrument",
    ),
    KeyBinding::action(
        BindingInput::Character('4'),
        GuiAction::Instrument(3),
        "1-5",
        "instrument",
    ),
    KeyBinding::action(
        BindingInput::Character('5'),
        GuiAction::Instrument(4),
        "1-5",
        "instrument",
    ),
];

const PALETTE_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::PaletteLeft,
        "h/l arrows",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::PaletteRight,
        "h/l arrows",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::PaletteLeft,
        "h/l arrows",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::PaletteRight,
        "h/l arrows",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::PaletteDown,
        "j/k arrows",
        "module",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::PaletteUp,
        "j/k arrows",
        "module",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::PaletteDown,
        "j/k arrows",
        "module",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::PaletteUp,
        "j/k arrows",
        "module",
    ),
    KeyBinding::action(
        BindingInput::Character('!'),
        GuiAction::Palette(ModuleCategory::Source),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('@'),
        GuiAction::Palette(ModuleCategory::Shape),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('#'),
        GuiAction::Palette(ModuleCategory::Filter),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('$'),
        GuiAction::Palette(ModuleCategory::Effect),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('%'),
        GuiAction::Palette(ModuleCategory::Logic),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('^'),
        GuiAction::Palette(ModuleCategory::Routing),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('&'),
        GuiAction::Palette(ModuleCategory::Composition),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Character('*'),
        GuiAction::Palette(ModuleCategory::Output),
        "! ... *",
        "category",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "n/enter",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('n'),
        GuiAction::OpenPalette,
        "n/enter",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('/'),
        GuiAction::Search,
        "/",
        "search",
    ),
    KeyBinding::action(
        BindingInput::Character('i'),
        GuiAction::Cancel,
        "i/esc",
        "close",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "i/esc",
        "close",
    ),
];

const PALETTE_SEARCH_BINDINGS: &[KeyBinding] = &[
    KeyBinding::text("type", "filter"),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "j/k arrows",
        "choose",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "j/k arrows",
        "choose",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "j/k arrows",
        "choose",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "j/k arrows",
        "choose",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Backspace),
        GuiAction::Backspace,
        "backspace",
        "delete",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "close",
    ),
];

const MOVE_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('H'),
        GuiAction::LeftFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('J'),
        GuiAction::DownFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('K'),
        GuiAction::UpFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('L'),
        GuiAction::RightFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('m'),
        GuiAction::Move,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('y'),
        GuiAction::Copy,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('p'),
        GuiAction::EditComposition,
        "p",
        "composition",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
];

const COPY_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('H'),
        GuiAction::LeftFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('J'),
        GuiAction::DownFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('K'),
        GuiAction::UpFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('L'),
        GuiAction::RightFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('m'),
        GuiAction::Move,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('y'),
        GuiAction::Copy,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
];

const EDIT_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "j/k arrows",
        "param",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "j/k arrows",
        "param",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "j/k arrows",
        "param",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "j/k arrows",
        "param",
    ),
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Character('H'),
        GuiAction::LeftFast,
        "H/L",
        "fast value",
    ),
    KeyBinding::action(
        BindingInput::Character('L'),
        GuiAction::RightFast,
        "H/L",
        "fast value",
    ),
    KeyBinding::action(
        BindingInput::Character('t'),
        GuiAction::TypeValue,
        "t",
        "type",
    ),
    KeyBinding::action(
        BindingInput::Character(';'),
        GuiAction::TogglePort,
        ";",
        "port",
    ),
    KeyBinding::action(
        BindingInput::Character('u'),
        GuiAction::CycleUnit,
        "u",
        "unit",
    ),
    KeyBinding::action(
        BindingInput::Character('s'),
        GuiAction::CycleStep,
        "s",
        "step",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter",
        "open",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::TogglePlay,
        "space",
        "play",
    ),
    KeyBinding::action(
        BindingInput::Character('i'),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
];

const ADSR_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "j/k arrows",
        "field",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "j/k arrows",
        "field",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "j/k arrows",
        "field",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "j/k arrows",
        "field",
    ),
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::ValueDown,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::ValueUp,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::ValueDown,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::ValueUp,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Character('H'),
        GuiAction::ValueDownFast,
        "H/L",
        "fast value",
    ),
    KeyBinding::action(
        BindingInput::Character('L'),
        GuiAction::ValueUpFast,
        "H/L",
        "fast value",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::TogglePlay,
        "space",
        "play",
    ),
    KeyBinding::action(
        BindingInput::Character('i'),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
];

const ENV_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "h/l arrows",
        "point",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "h/l arrows",
        "point",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "h/l arrows",
        "point",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "h/l arrows",
        "point",
    ),
    KeyBinding::action(BindingInput::Character('m'), GuiAction::Move, "m", "move"),
    KeyBinding::action(
        BindingInput::Character('n'),
        GuiAction::AddPoint,
        "n/.",
        "add/delete",
    ),
    KeyBinding::action(
        BindingInput::Character('.'),
        GuiAction::DeletePoint,
        "n/.",
        "add/delete",
    ),
    KeyBinding::action(
        BindingInput::Character('c'),
        GuiAction::ToggleCurve,
        "c",
        "curve",
    ),
    KeyBinding::action(
        BindingInput::Character('s'),
        GuiAction::CycleStep,
        "s",
        "step",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::TogglePlay,
        "space",
        "play",
    ),
    KeyBinding::action(
        BindingInput::Character('i'),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
];

const ENV_MOVE_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('H'),
        GuiAction::LeftFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('J'),
        GuiAction::DownFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('K'),
        GuiAction::UpFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('L'),
        GuiAction::RightFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('s'),
        GuiAction::CycleStep,
        "s",
        "step",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::TogglePlay,
        "space",
        "play",
    ),
    KeyBinding::action(
        BindingInput::Character('m'),
        GuiAction::Move,
        "m/enter",
        "done",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "m/enter",
        "done",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
];

const PROBE_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "hjkl/arrows",
        "length",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "hjkl/arrows",
        "length",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "hjkl/arrows",
        "length",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "hjkl/arrows",
        "length",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "hjkl/arrows",
        "length",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "hjkl/arrows",
        "length",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "hjkl/arrows",
        "length",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "hjkl/arrows",
        "length",
    ),
    KeyBinding::action(
        BindingInput::Character('r'),
        GuiAction::Delete,
        "r/.",
        "reset",
    ),
    KeyBinding::action(
        BindingInput::Character('.'),
        GuiAction::Delete,
        "r/.",
        "reset",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::TogglePlay,
        "space",
        "play",
    ),
    KeyBinding::action(
        BindingInput::Character('i'),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
];

const SAMPLE_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "h/l arrows",
        "pan",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "h/l arrows",
        "pan",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "h/l arrows",
        "pan",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "h/l arrows",
        "pan",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "j/k arrows",
        "zoom",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "j/k arrows",
        "zoom",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "j/k arrows",
        "zoom",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "j/k arrows",
        "zoom",
    ),
    KeyBinding::action(
        BindingInput::Character('r'),
        GuiAction::Delete,
        "r/.",
        "reset",
    ),
    KeyBinding::action(
        BindingInput::Character('.'),
        GuiAction::Delete,
        "r/.",
        "reset",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::TogglePlay,
        "space",
        "play",
    ),
    KeyBinding::action(
        BindingInput::Character('i'),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "i/esc",
        "done",
    ),
];

const SELECT_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "hjkl/arrows",
        "resize",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "hjkl/arrows",
        "resize",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "hjkl/arrows",
        "resize",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "hjkl/arrows",
        "resize",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "hjkl/arrows",
        "resize",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "hjkl/arrows",
        "resize",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "hjkl/arrows",
        "resize",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "hjkl/arrows",
        "resize",
    ),
    KeyBinding::action(
        BindingInput::Character('H'),
        GuiAction::LeftFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('J'),
        GuiAction::DownFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('K'),
        GuiAction::UpFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('L'),
        GuiAction::RightFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter/m",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('m'),
        GuiAction::Move,
        "enter/m",
        "move",
    ),
    KeyBinding::action(BindingInput::Character('y'), GuiAction::Copy, "y", "copy"),
    KeyBinding::action(
        BindingInput::Character('r'),
        GuiAction::Delete,
        "r/.",
        "delete",
    ),
    KeyBinding::action(
        BindingInput::Character('.'),
        GuiAction::Delete,
        "r/.",
        "delete",
    ),
    KeyBinding::action(
        BindingInput::Character(','),
        GuiAction::Cancel,
        ",/esc",
        "cancel",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        ",/esc",
        "cancel",
    ),
];

const SELECT_MOVE_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('H'),
        GuiAction::LeftFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('J'),
        GuiAction::DownFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('K'),
        GuiAction::UpFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('L'),
        GuiAction::RightFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('m'),
        GuiAction::Move,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('y'),
        GuiAction::Copy,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('p'),
        GuiAction::EditComposition,
        "p",
        "composition",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
];

const COPY_SELECTION_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "hjkl/arrows",
        "move",
    ),
    KeyBinding::action(
        BindingInput::Character('H'),
        GuiAction::LeftFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('J'),
        GuiAction::DownFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('K'),
        GuiAction::UpFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Character('L'),
        GuiAction::RightFast,
        "HJKL",
        "jump",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('m'),
        GuiAction::Move,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Character('y'),
        GuiAction::Copy,
        "enter/m/y",
        "place",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
];

const TEXT_BINDINGS: &[KeyBinding] = &[
    KeyBinding::text("type", "text"),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter",
        "confirm",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Backspace),
        GuiAction::Backspace,
        "backspace",
        "delete",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Delete),
        GuiAction::DeleteChar,
        "delete",
        "delete",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "left/right",
        "cursor",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "left/right",
        "cursor",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Home),
        GuiAction::TextStart,
        "home/end",
        "cursor",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::End),
        GuiAction::TextEnd,
        "home/end",
        "cursor",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::InputChar(' '),
        "type",
        "text",
    ),
];

const EXPORT_TEXT_BINDINGS: &[KeyBinding] = &[
    KeyBinding::text("type", "text"),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter",
        "confirm",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Backspace),
        GuiAction::Backspace,
        "backspace",
        "delete",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Delete),
        GuiAction::DeleteChar,
        "delete",
        "delete",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "left/right",
        "cursor",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "left/right",
        "cursor",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "up/down",
        "loops",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "up/down",
        "loops",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Home),
        GuiAction::TextStart,
        "home/end",
        "cursor",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::End),
        GuiAction::TextEnd,
        "home/end",
        "cursor",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::InputChar(' '),
        "type",
        "text",
    ),
];

const TRACK_TEXT_BINDINGS: &[KeyBinding] = &[
    KeyBinding::text("type", "text"),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter",
        "confirm",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
];

const CONFIRM_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('y'),
        GuiAction::Confirm,
        "Y/y/enter",
        "confirm",
    ),
    KeyBinding::action(
        BindingInput::Character('Y'),
        GuiAction::Confirm,
        "Y/y/enter",
        "confirm",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "Y/y/enter",
        "confirm",
    ),
    KeyBinding::action(
        BindingInput::Character('n'),
        GuiAction::Cancel,
        "N/n/esc",
        "back",
    ),
    KeyBinding::action(
        BindingInput::Character('N'),
        GuiAction::Cancel,
        "N/n/esc",
        "back",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "N/n/esc",
        "back",
    ),
];

const LOAD_CONFIRM_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter",
        "save",
    ),
    KeyBinding::action(
        BindingInput::Character('d'),
        GuiAction::Delete,
        "d",
        "don't save",
    ),
    KeyBinding::action(
        BindingInput::Character('D'),
        GuiAction::Delete,
        "d",
        "don't save",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
];

const SAVE_CONFIRM_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "enter",
        "overwrite",
    ),
    KeyBinding::action(
        BindingInput::Character('a'),
        GuiAction::SaveAs,
        "a",
        "save as",
    ),
    KeyBinding::action(
        BindingInput::Character('A'),
        GuiAction::SaveAs,
        "a",
        "save as",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "esc",
        "cancel",
    ),
];

const SETTINGS_BINDINGS: &[KeyBinding] = &[
    KeyBinding::action(
        BindingInput::Character('j'),
        GuiAction::Down,
        "j/k arrows",
        "field",
    ),
    KeyBinding::action(
        BindingInput::Character('k'),
        GuiAction::Up,
        "j/k arrows",
        "field",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowDown),
        GuiAction::Down,
        "j/k arrows",
        "field",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowUp),
        GuiAction::Up,
        "j/k arrows",
        "field",
    ),
    KeyBinding::action(
        BindingInput::Character('h'),
        GuiAction::Left,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Character('l'),
        GuiAction::Right,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowLeft),
        GuiAction::Left,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::ArrowRight),
        GuiAction::Right,
        "h/l arrows",
        "value",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Space),
        GuiAction::TogglePlay,
        "space",
        "play",
    ),
    KeyBinding::action(
        BindingInput::Character('s'),
        GuiAction::TrackSettings,
        "s/enter/esc",
        "done",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Enter),
        GuiAction::Confirm,
        "s/enter/esc",
        "done",
    ),
    KeyBinding::action(
        BindingInput::Named(NamedKey::Escape),
        GuiAction::Cancel,
        "s/enter/esc",
        "done",
    ),
];

fn bindings(state: &GuiState) -> &'static [KeyBinding] {
    if state.palette_searching() {
        return PALETTE_SEARCH_BINDINGS;
    }
    match state.mode() {
        Mode::Normal => NORMAL_BINDINGS,
        Mode::Palette => PALETTE_BINDINGS,
        Mode::Move { .. } => MOVE_BINDINGS,
        Mode::Copy { .. } => COPY_BINDINGS,
        Mode::Edit { .. } => EDIT_BINDINGS,
        Mode::AdsrEdit { .. } => ADSR_BINDINGS,
        Mode::EnvEdit { editing: true, .. } => ENV_MOVE_BINDINGS,
        Mode::EnvEdit { .. } => ENV_BINDINGS,
        Mode::ProbeEdit { .. } => PROBE_BINDINGS,
        Mode::SampleView { .. } => SAMPLE_BINDINGS,
        Mode::Select { .. } => SELECT_BINDINGS,
        Mode::SelectMove { .. } => SELECT_MOVE_BINDINGS,
        Mode::CopySelection { .. } => COPY_SELECTION_BINDINGS,
        Mode::ValueInput { .. } => TEXT_BINDINGS,
        Mode::ExportPrompt => EXPORT_TEXT_BINDINGS,
        Mode::TrackPrompt => TRACK_TEXT_BINDINGS,
        Mode::LoadConfirm => LOAD_CONFIRM_BINDINGS,
        Mode::SaveConfirm => SAVE_CONFIRM_BINDINGS,
        Mode::ExportConfirm | Mode::QuitConfirm => CONFIRM_BINDINGS,
        Mode::TrackSettings { .. } => SETTINGS_BINDINGS,
    }
}

fn binding_inputs() -> Vec<BindingInput> {
    let mut inputs = Vec::new();
    for bindings in [
        NORMAL_BINDINGS,
        PALETTE_BINDINGS,
        PALETTE_SEARCH_BINDINGS,
        MOVE_BINDINGS,
        COPY_BINDINGS,
        EDIT_BINDINGS,
        ADSR_BINDINGS,
        ENV_BINDINGS,
        ENV_MOVE_BINDINGS,
        PROBE_BINDINGS,
        SAMPLE_BINDINGS,
        SELECT_BINDINGS,
        SELECT_MOVE_BINDINGS,
        COPY_SELECTION_BINDINGS,
        TEXT_BINDINGS,
        EXPORT_TEXT_BINDINGS,
        TRACK_TEXT_BINDINGS,
        CONFIRM_BINDINGS,
        LOAD_CONFIRM_BINDINGS,
        SAVE_CONFIRM_BINDINGS,
        SETTINGS_BINDINGS,
    ] {
        for binding in bindings {
            if binding.input != BindingInput::Text && !inputs.contains(&binding.input) {
                inputs.push(binding.input);
            }
        }
    }
    for character in TEXT_INPUT_CHARS.chars() {
        let input = BindingInput::Character(character);
        if !inputs.contains(&input) {
            inputs.push(input);
        }
    }
    inputs
}

fn binding_key(id: u64, input: BindingInput, grid_size: Option<GridViewSize>) -> Gesture<GuiState> {
    let key = match input {
        BindingInput::Character(character) => {
            gesture::key(id).key(Key::character(character.to_string()))
        }
        BindingInput::Named(named) => gesture::key(id).key(named),
        BindingInput::Text => gesture::key(id).key(Key::character("")),
    };
    key.run(move |state: &mut GuiState, _, event| {
        if event.phase == KeyPhase::Pressed {
            if let Some(size) = grid_size {
                state.set_grid_view_size(size);
            }
            apply_binding_input(state, input);
        }
    })
}

fn apply_binding_input(state: &mut GuiState, input: BindingInput) {
    let current = bindings(state);
    if let Some(binding) = current.iter().find(|binding| binding.input == input)
        && let BindingEffect::Action(action) = binding.effect
    {
        state.apply(action);
        return;
    }
    if current
        .iter()
        .any(|binding| matches!(binding.effect, BindingEffect::Text))
        && let BindingInput::Character(character) = input
        && !matches!(state.mode(), Mode::TrackPrompt)
    {
        state.apply(GuiAction::InputChar(character));
    }
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
        GuiAction::OpenModules => 72,
        GuiAction::SaveModule => 73,
        GuiAction::TrackSettings => 67,
        GuiAction::TrackEdit => 68,
        GuiAction::Search => 69,
        GuiAction::EditComposition => 70,
        GuiAction::ExitComposition => 71,
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
                    state.selected_palette_label()
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
        Mode::LoadConfirm => "Open".to_string(),
        Mode::SaveConfirm => "Save".to_string(),
        Mode::ExportPrompt => "Export".to_string(),
        Mode::ExportConfirm => "Overwrite export".to_string(),
        Mode::TrackPrompt => "Track".to_string(),
        Mode::TrackSettings { parameter } => format!("Settings {}", parameter + 1),
    }
}

fn cell_id(position: GridPos) -> u64 {
    if position.x < 10 && position.y < 10 {
        10_000 + position.y as u64 * 100 + position.x as u64 * 10
    } else {
        700_000 + position.y as u64 * 1_000 + position.x as u64 * 10
    }
}

fn module_color(category: ModuleCategory) -> Color {
    match category {
        ModuleCategory::Source => Color::from_rgb8(34, 139, 132),
        ModuleCategory::Shape => Color::from_rgb8(188, 112, 42),
        ModuleCategory::Filter => Color::from_rgb8(80, 125, 202),
        ModuleCategory::Effect => Color::from_rgb8(154, 86, 178),
        ModuleCategory::Logic => Color::from_rgb8(190, 76, 91),
        ModuleCategory::Routing => Color::from_rgb8(110, 128, 84),
        ModuleCategory::Composition => Color::from_rgb8(96, 112, 140),
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
            wire_segment(GridPos::new(0, 0), Orientation::Right, GridPos::new(2, 0)),
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
            wire_segment(GridPos::new(0, 0), Orientation::Down, GridPos::new(0, 2)),
            WireSegment {
                x: 13.,
                y: 22.5,
                width: 4.,
                height: 51.,
            }
        );
    }

    #[test]
    fn grid_position_accounts_for_balanced_padding() {
        let area = Area::new(0., 0., 100., 70.);
        let size = GridViewSize::new(3, 2);

        assert_eq!(
            grid_position(Point::new(1., 1.), GridPos::new(0, 0), size, area),
            None
        );
        assert_eq!(
            grid_position(Point::new(17., 4.), GridPos::new(0, 0), size, area),
            Some(GridPos::new(0, 0))
        );
        assert_eq!(
            grid_position(Point::new(98., 68.), GridPos::new(0, 0), size, area),
            None
        );
    }

    #[test]
    fn visible_axis_does_not_build_offscreen_cells() {
        assert_eq!(visible_axis(240, 12, 400), 240..252);
        assert_eq!(visible_axis(395, 12, 400), 395..400);
    }

    #[test]
    fn sample_path_reflects_loaded_amplitudes() {
        let area = Area::new(0., 0., 100., 100.);
        assert_ne!(
            sample_path(area, &[-1.0, 1.0]),
            sample_path(area, &[1.0, -1.0])
        );
    }
}
