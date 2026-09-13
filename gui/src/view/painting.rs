use super::*;
use crate::model::painting::{Brush, PaintPanel};

pub(super) fn painting_panel<'a>(state: &'a GuiState, app: &mut PaneState) -> View<'a, GuiState> {
    let editor = &state.painting;
    let mode_name = editor.tool().name();
    let sequence = state.sequence().clone();
    let low_pitch = editor.low_pitch;
    let span = editor.span;
    let offset = editor.offset;
    let selected = editor.selected;
    let playhead = state.sequence_playhead();
    let preview = editor.preview().to_vec();
    let pointer = editor.pointer.filter(|_| editor.panel.is_none());
    let radius = if matches!(editor.tool(), Brush::Draw | Brush::Select) {
        5.0
    } else {
        editor.radius.value
    };
    let cursor_color = if editor.tool() == Brush::Expression {
        expression_color(editor.expression.value)
    } else if editor.tool() == Brush::Erase {
        Color::from_rgb8(255, 100, 100)
    } else {
        fg()
    };
    let graph = draw(move |area, app: &mut PaneState| {
        let mut layers = vec![rect(71_000).fill(field()).build(app).inert()];
        for note in 0..=127 {
            let in_scale = state.sequence_pitch(note as f32) == note as f32;
            let y = 1.0 - (note as f32 - low_pitch) / 36.0;
            if !(0.0..=1.0).contains(&y) {
                continue;
            }
            layers.push(
                path(71_010 + note, move |area| {
                    let mut path = BezPath::new();
                    let y = area.y + y * area.height;
                    path.move_to((area.x as f64, y as f64));
                    path.line_to(((area.x + area.width) as f64, y as f64));
                    path
                })
                .stroke(
                    if in_scale {
                        accent().with_alpha(0.45)
                    } else {
                        line().with_alpha(0.35)
                    },
                    Stroke::new(if in_scale { 1.5 } else { 1. }),
                )
                .build(app)
                .inert(),
            );
        }
        let first = (offset * 4.0).ceil() as u32;
        let last = ((offset + span) * 4.0).floor() as u32;
        for beat in first..=last {
            let x = (beat as f32 / 4.0 - offset) / span;
            layers.push(
                path(71_400 + beat as u64, move |area| {
                    let mut path = BezPath::new();
                    let x = area.x + x * area.width;
                    path.move_to((x as f64, area.y as f64));
                    path.line_to((x as f64, (area.y + area.height) as f64));
                    path
                })
                .stroke(line().with_alpha(0.6), Stroke::new(1.))
                .build(app)
                .inert(),
            );
        }
        for (index, stroke) in sequence.strokes().iter().enumerate() {
            for (segment, pair) in stroke.points().windows(2).enumerate() {
                let a = pair[0];
                let b = pair[1];
                if b[0] < offset || a[0] > offset + span {
                    continue;
                }
                layers.push(
                    path(
                        100_000_000 + index as u64 * 1_000_000 + segment as u64,
                        move |area| stroke_path(area, &[a, b], offset, span, low_pitch),
                    )
                    .stroke(
                        expression_color((a[2] + b[2]) * 0.5),
                        Stroke::new(if selected == Some(index) { 5.0 } else { 3.0 }),
                    )
                    .build(app)
                    .inert(),
                );
            }
        }
        if (offset..=offset + span).contains(&playhead) {
            layers.push(
                path(71_898, move |area| {
                    let mut path = BezPath::new();
                    let x = area.x + (playhead - offset) / span * area.width;
                    path.move_to((x as f64, area.y as f64));
                    path.line_to((x as f64, (area.y + area.height) as f64));
                    path
                })
                .stroke(fg().with_alpha(0.75), Stroke::new(1.5))
                .build(app)
                .inert(),
            );
        }
        let points = preview.clone();
        layers.push(
            path(71_900, move |area| {
                stroke_path(area, &points, offset, span, low_pitch)
            })
            .stroke(fg(), Stroke::new(3.))
            .build(app)
            .inert(),
        );
        let width = area.width;
        let height = area.height;
        layers.push(
            rect(71_901)
                .fill(Color::TRANSPARENT)
                .view()
                .gesture(
                    gesture::hover(71_904).run(|state: &mut GuiState, _, hovered| {
                        if !hovered {
                            state.painting.pointer = None;
                        }
                    }),
                )
                .gesture(gesture::scroll(71_907).modifiers(Modifier::Control).run(
                    |state: &mut GuiState, _, delta| {
                        if state.painting.panel.is_some() {
                            return;
                        }
                        let center = state.painting.offset + state.painting.span / 2.0;
                        state.painting.span = (state.painting.span * (delta.y * 0.002).exp())
                            .clamp(1.0 / 16.0, state.sequence().bars() as f32);
                        state.painting.offset = (center - state.painting.span / 2.0)
                            .clamp(0.0, state.sequence().bars() as f32 - state.painting.span);
                    },
                ))
                .gesture(
                    gesture::scroll(71_908)
                        .modifiers(!ModifierPredicate::from(Modifier::Control))
                        .run(move |state: &mut GuiState, _, delta| {
                            if state.painting.panel.is_some() {
                                return;
                            }
                            state.painting.low_pitch = (state.painting.low_pitch
                                + delta.y * 36.0 / height.max(1.0))
                            .clamp(0.0, 91.0);
                            state.painting.offset = (state.painting.offset
                                - delta.x * state.painting.span / width.max(1.0))
                            .clamp(
                                0.0,
                                (state.sequence().bars() as f32 - state.painting.span).max(0.0),
                            );
                        }),
                )
                .gesture(gesture::drag(71_902).button(MouseButton::Left).run(
                    move |state: &mut GuiState, _, drag| {
                        let (phase, point) = match drag {
                            DragPhase::Began { start, .. } => (EnvPointerPhase::Start, start),
                            DragPhase::Updated { current, .. } => (EnvPointerPhase::Drag, current),
                            DragPhase::Completed { current, .. } => (EnvPointerPhase::End, current),
                        };
                        state.painting.pointer = match phase {
                            EnvPointerPhase::End => None,
                            _ => Some([point.x as f32, point.y as f32]),
                        };
                        state.paint_pointer(
                            phase,
                            point.x as f32 / width.max(1.0),
                            point.y as f32 / height.max(1.0),
                            width,
                            height,
                        );
                    },
                ))
                .gesture(gesture::click(71_903).button(MouseButton::Left).run(
                    move |state: &mut GuiState, _, event| {
                        if matches!(event.state, ClickPhase::Completed) {
                            let point = event.location.local();
                            state.paint_pointer(
                                EnvPointerPhase::Start,
                                point.x as f32 / width.max(1.0),
                                point.y as f32 / height.max(1.0),
                                width,
                                height,
                            );
                            state.paint_pointer(
                                EnvPointerPhase::End,
                                point.x as f32 / width.max(1.0),
                                point.y as f32 / height.max(1.0),
                                width,
                                height,
                            );
                        }
                    },
                ))
                .build(app),
        );
        let mut elements = stack(layers).draw(area, app);
        if editor.tool() == Brush::Select
            && editor.panel.is_none()
            && let Some(stroke) = selected.and_then(|index| sequence.strokes().get(index))
        {
            let [low, high] = crate::model::painting::stroke_bounds(stroke);
            let left = ((low[0] - offset) / span * area.width).clamp(0.0, area.width);
            let right = ((high[0] - offset) / span * area.width).clamp(0.0, area.width);
            let top = ((1.0 - (high[1] - low_pitch) / 36.0) * area.height).clamp(0.0, area.height);
            let bottom =
                ((1.0 - (low[1] - low_pitch) / 36.0) * area.height).clamp(0.0, area.height);
            if right > left && bottom > top {
                for (index, handle) in [
                    [0, 0],
                    [-1, 1],
                    [0, 1],
                    [1, 1],
                    [-1, 0],
                    [1, 0],
                    [-1, -1],
                    [0, -1],
                    [1, -1],
                ]
                .into_iter()
                .enumerate()
                {
                    let id = 75_000 + index as u64 * 3;
                    let bounds = if handle == [0, 0] {
                        Area::new(area.x + left, area.y + top, right - left, bottom - top)
                    } else {
                        let x = match handle[0] {
                            -1 => left,
                            1 => right,
                            _ => (left + right) * 0.5,
                        };
                        let y = match handle[1] {
                            -1 => bottom,
                            1 => top,
                            _ => (top + bottom) * 0.5,
                        };
                        Area::new(area.x + x - 4.0, area.y + y - 4.0, 8.0, 8.0)
                    };
                    elements.extend(
                        rect(id)
                            .fill(if handle == [0, 0] {
                                Color::TRANSPARENT
                            } else {
                                accent()
                            })
                            .stroke(accent(), Stroke::new(1.0))
                            .view()
                            .gesture(gesture::drag(id + 1).button(MouseButton::Left).run(
                                move |state: &mut GuiState, _, drag| {
                                    let (phase, start, current) = match drag {
                                        DragPhase::Began { start, .. } => {
                                            (EnvPointerPhase::Start, start, start)
                                        }
                                        DragPhase::Updated { start, current, .. } => {
                                            (EnvPointerPhase::Drag, start, current)
                                        }
                                        DragPhase::Completed { start, current, .. } => {
                                            (EnvPointerPhase::End, start, current)
                                        }
                                    };
                                    state.transform_stroke(
                                        phase,
                                        handle,
                                        [
                                            (current.x - start.x) as f32 * span / area.width,
                                            -(current.y - start.y) as f32 * 36.0 / area.height,
                                        ],
                                    );
                                },
                            ))
                            .build(app)
                            .draw(bounds, app),
                    );
                }
            }
        }
        if let Some([x, y]) = pointer.or_else(|| {
            (!matches!(editor.tool(), Brush::Draw | Brush::Select) && editor.panel.is_none())
                .then_some([radius + 12.0, area.height - radius - 12.0])
        }) {
            elements.extend(
                circle(71_905)
                    .stroke(cursor_color, Stroke::new(1.5))
                    .view()
                    .build(app)
                    .inert()
                    .draw(
                        Area::new(
                            area.x + x - radius,
                            area.y + y - radius,
                            radius * 2.0,
                            radius * 2.0,
                        ),
                        app,
                    ),
            );
            elements.extend(
                text(71_906, mode_name)
                    .font_size(11)
                    .fill(cursor_color)
                    .view()
                    .build(app)
                    .inert()
                    .draw(
                        Area::new(
                            (area.x + x + radius + 8.0).min(area.x + area.width - 90.0),
                            (area.y + y + 10.0).min(area.y + area.height - 16.0),
                            90.0,
                            16.0,
                        ),
                        app,
                    ),
            );
        }
        for note in (0..=120).step_by(12) {
            let y = 1.0 - (note as f32 - low_pitch) / 36.0;
            if !(0.0..=1.0).contains(&y) {
                continue;
            }
            elements.extend(
                text(71_200 + note, format!("C{}", note as i32 / 12 - 1))
                    .font_size(11)
                    .fill(quiet())
                    .view()
                    .build(app)
                    .inert()
                    .draw(
                        Area::new(
                            area.x + 3.0,
                            area.y + (y * area.height - 12.0).max(0.0),
                            25.0,
                            12.0,
                        ),
                        app,
                    ),
            );
        }
        elements
    });
    let mut canvas = vec![graph.expand()];
    if let Some(disclosure) = editor.panel {
        let quantization = vec![
            paint_button(
                70_030,
                format!("Pitch snap {}", if editor.snap { "on" } else { "off" }),
                editor.snap,
                |state| state.painting.snap = !state.painting.snap,
                app,
            ),
            paint_button(
                70_034,
                format!(
                    "Time snap {}",
                    if editor.time_snap { "1/16" } else { "off" }
                ),
                editor.time_snap,
                |state| state.painting.time_snap = !state.painting.time_snap,
                app,
            ),
            paint_button(
                70_038,
                if editor.selected.is_some() {
                    "Snap selected stroke"
                } else {
                    "Snap all strokes"
                }
                .into(),
                false,
                GuiState::snap_paint,
                app,
            ),
        ];
        let paint = vec![
            paint_slider(
                72_000,
                format!("Expression {:+.2}", editor.expression.value),
                binding!(state.painting.expression),
                (-1.0, 1.0),
                app,
            ),
            paint_slider(
                72_010,
                format!("Strength {:.0}%", editor.strength.value * 100.0),
                binding!(state.painting.strength),
                (0.01, 1.0),
                app,
            ),
            paint_slider(
                72_020,
                format!("Radius {:.0} px", editor.radius.value),
                binding!(state.painting.radius),
                (8.0, 96.0),
                app,
            ),
        ];
        let navigation = vec![
            paint_button(
                70_062,
                "Earlier".into(),
                false,
                |state| {
                    state.painting.offset =
                        (state.painting.offset - state.painting.span / 2.0).max(0.0)
                },
                app,
            ),
            paint_button(
                70_066,
                "Later".into(),
                false,
                |state| {
                    state.painting.offset = (state.painting.offset + state.painting.span / 2.0)
                        .min((state.sequence().bars() as f32 - state.painting.span).max(0.0))
                },
                app,
            ),
            paint_button(
                70_070,
                "Zoom +".into(),
                false,
                |state| state.painting.span = (state.painting.span / 2.0).max(1.0 / 16.0),
                app,
            ),
            paint_button(
                70_074,
                "Zoom −".into(),
                false,
                |state| {
                    state.painting.span =
                        (state.painting.span * 2.0).min(state.sequence().bars() as f32);
                    state.painting.offset = state
                        .painting
                        .offset
                        .min(state.sequence().bars() as f32 - state.painting.span);
                },
                app,
            ),
            paint_button(
                70_078,
                "Oct −".into(),
                false,
                |state| state.painting.low_pitch = (state.painting.low_pitch - 12.0).max(0.0),
                app,
            ),
            paint_button(
                70_082,
                "Oct +".into(),
                false,
                |state| state.painting.low_pitch = (state.painting.low_pitch + 12.0).min(91.0),
                app,
            ),
        ];
        let outside = gesture::click(70_141)
            .button(MouseButton::Left)
            .anywhere()
            .run(|state: &mut GuiState, _, event| {
                if matches!(event.state, ClickPhase::Completed) {
                    state.painting.panel = None;
                }
            });
        canvas.push(
            rect(70_140)
                .fill(Color::TRANSPARENT)
                .view()
                .gesture(outside.clone())
                .build(app),
        );
        let (title, mut controls): (&str, Vec<View<'a, GuiState>>) = match disclosure {
            PaintPanel::Options(tool) => {
                let controls = match tool {
                    Brush::Draw => quantization.into_iter().take(2).collect(),
                    Brush::Select | Brush::Quantize => quantization
                        .into_iter()
                        .skip(1)
                        .chain(
                            paint
                                .into_iter()
                                .skip(1)
                                .filter(|_| tool == Brush::Quantize),
                        )
                        .collect(),
                    Brush::Shape => paint.into_iter().skip(1).collect(),
                    Brush::Expression => paint,
                    Brush::Erase => paint.into_iter().skip(2).collect(),
                };
                (tool.name(), controls)
            }
            PaintPanel::View => ("Canvas view", navigation),
        };
        controls.insert(
            0,
            text(70_110, title)
                .font_size(14)
                .fill(fg())
                .view()
                .build(app),
        );
        let height = controls.len() as f32 * 40.0 + 20.0;
        let mut popup = stack(vec![
            rect(70_118)
                .fill(panel())
                .stroke(line(), Stroke::new(1.))
                .corner_rounding(8.)
                .view()
                .occlude(&outside)
                .build(app),
            column_spaced(8., controls).pad(12.),
        ]);
        canvas.push(draw(move |area, app| {
            popup.draw(
                Area::new(
                    area.x
                        + match disclosure {
                            PaintPanel::Options(_) => 128.0_f32.min((area.width - 268.0).max(0.0)),
                            PaintPanel::View => (area.width - 268.0).max(0.0),
                        },
                    area.y + 8.0,
                    260.0_f32.min(area.width),
                    height,
                ),
                app,
            )
        }));
    }
    let bar = row_spaced_aligned(
        6.,
        Align::TopLeading,
        vec![
            dropdown(
                70_120,
                binding!(state.painting.brush),
                vec![
                    Brush::Draw,
                    Brush::Select,
                    Brush::Shape,
                    Brush::Quantize,
                    Brush::Expression,
                    Brush::Erase,
                ],
                move |item, app| {
                    text(
                        74_000 + item.index as u64,
                        if item.expanded {
                            item.value.name().to_string()
                        } else {
                            mode_name.to_string()
                        },
                    )
                    .font_size(13)
                    .fill(fg())
                    .view()
                    .build(app)
                    .pad_x(9.)
                    .pad_y(6.)
                    .height(28.)
                },
            )
            .background(|_, app| {
                rect(74_090)
                    .fill(field())
                    .stroke(line(), Stroke::new(1.0))
                    .corner_rounding(7.)
                    .build(app)
            })
            .build(app)
            .width(110.),
            paint_button(
                70_124,
                "Settings...".into(),
                matches!(editor.panel, Some(PaintPanel::Options(_))),
                |state| {
                    state.painting.panel =
                        (!matches!(state.painting.panel, Some(PaintPanel::Options(_))))
                            .then_some(PaintPanel::Options(state.painting.tool()));
                },
                app,
            )
            .width(90.),
            space().expand(),
            paint_button(
                70_128,
                "View...".into(),
                editor.panel == Some(PaintPanel::View),
                |state| {
                    state.painting.panel = (state.painting.panel != Some(PaintPanel::View))
                        .then_some(PaintPanel::View);
                },
                app,
            )
            .width(65.),
            dropdown(
                70_132,
                binding!(state.painting.loop_menu),
                {
                    let minimum = state
                        .sequence()
                        .strokes()
                        .iter()
                        .map(|stroke| stroke.points().last().unwrap()[0].ceil() as u16)
                        .max()
                        .unwrap_or(1)
                        .max(1);
                    let mut options = vec![
                        1,
                        2,
                        4,
                        8,
                        16,
                        32,
                        64,
                        128,
                        256,
                        512,
                        1024,
                        state.sequence().bars(),
                    ];
                    options.retain(|bars| *bars >= minimum);
                    options.sort_unstable();
                    options.dedup();
                    options
                },
                move |item, app| {
                    text(
                        74_100 + item.index as u64,
                        format!(
                            "{} bar loop",
                            if item.expanded {
                                *item.value
                            } else {
                                state.sequence().bars()
                            }
                        ),
                    )
                    .font_size(13)
                    .fill(fg())
                    .view()
                    .build(app)
                    .pad_x(9.)
                    .pad_y(6.)
                    .height(28.)
                },
            )
            .background(|_, app| {
                rect(74_190)
                    .fill(field())
                    .stroke(line(), Stroke::new(1.0))
                    .corner_rounding(7.)
                    .build(app)
            })
            .on_select(|state, _, bars| state.resize_loop(*bars))
            .build(app)
            .width(100.),
        ],
    )
    .height(28.);
    column_spaced(8., vec![bar, stack(canvas).expand()]).expand()
}

fn expression_color(value: f32) -> Color {
    let middle = [210.0, 219.0, 225.0];
    let end = if value < 0.0 {
        [77.0, 152.0, 255.0]
    } else {
        [255.0, 153.0, 64.0]
    };
    let rgb: [u8; 3] =
        std::array::from_fn(|i| (middle[i] + (end[i] - middle[i]) * value.abs()) as u8);
    Color::from_rgb8(rgb[0], rgb[1], rgb[2])
}

fn stroke_path(area: Area, points: &[[f32; 3]], offset: f32, span: f32, low_pitch: f32) -> BezPath {
    let mut path = BezPath::new();
    for pair in points.windows(2) {
        let a = [
            (pair[0][0] - offset) / span,
            1.0 - (pair[0][1] - low_pitch) / 36.0,
        ];
        let b = [
            (pair[1][0] - offset) / span,
            1.0 - (pair[1][1] - low_pitch) / 36.0,
        ];
        let mut start = 0.0_f32;
        let mut end = 1.0_f32;
        for axis in 0..2 {
            let delta = b[axis] - a[axis];
            if delta == 0.0 {
                if !(0.0..=1.0).contains(&a[axis]) {
                    end = -1.0;
                }
            } else {
                let lo = -a[axis] / delta;
                let hi = (1.0 - a[axis]) / delta;
                start = start.max(lo.min(hi));
                end = end.min(lo.max(hi));
            }
        }
        if start <= end {
            for (index, fraction) in [start, end].into_iter().enumerate() {
                let x = area.x + (a[0] + (b[0] - a[0]) * fraction) * area.width;
                let y = area.y + (a[1] + (b[1] - a[1]) * fraction) * area.height;
                if index == 0 {
                    path.move_to((x as f64, y as f64));
                } else {
                    path.line_to((x as f64, y as f64));
                }
            }
        }
    }
    path
}

pub(super) fn paint_button<'a>(
    id: u64,
    label: String,
    selected: bool,
    action: impl Fn(&mut GuiState) + 'static,
    app: &mut PaneState,
) -> View<'a, GuiState> {
    stack(vec![
        rect(id)
            .fill(if selected { accent() } else { field() })
            .stroke(line(), Stroke::new(1.))
            .corner_rounding(7.)
            .view()
            .gesture(gesture::click(id + 1).button(MouseButton::Left).run(
                move |state: &mut GuiState, _, event| {
                    if matches!(event.state, ClickPhase::Completed) {
                        action(state);
                    }
                },
            ))
            .build(app),
        text(id + 2, label)
            .font_size(13)
            .fill(if selected { Color::BLACK } else { fg() })
            .view()
            .build(app)
            .pad_x(9.)
            .pad_y(6.)
            .inert(),
    ])
    .height(28.)
}

fn paint_slider<'a>(
    id: u64,
    label: String,
    state: (&SliderState, haven::Binding<GuiState, SliderState>),
    range: (f32, f32),
    app: &mut PaneState,
) -> View<'a, GuiState> {
    column_spaced(
        2.,
        vec![
            text(id + 4, label)
                .font_size(12)
                .fill(fg())
                .view()
                .build(app),
            slider(id, state)
                .range(range.0, range.1)
                .traveled_track(move |_, _, app| {
                    rect(id + 6).fill(accent()).corner_rounding(10.).build(app)
                })
                .build(app)
                .height(20.),
        ],
    )
    .height(40.)
}
