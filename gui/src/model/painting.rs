use super::*;
use brainwash::sequence::{Sequence, Stroke};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Brush {
    Draw,
    Select,
    Shape,
    Quantize,
    Expression,
    Erase,
}

impl Brush {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Draw => "Draw",
            Self::Select => "Select",
            Self::Shape => "Shape",
            Self::Quantize => "Quantize",
            Self::Expression => "Expression",
            Self::Erase => "Erase",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaintPanel {
    Options(Brush),
    View,
}

#[derive(Debug)]
pub(crate) struct Painting {
    pub open: bool,
    pub brush: haven::DropdownState<Brush>,
    pub loop_menu: haven::DropdownState<u16>,
    pub last_tap: Option<(Brush, std::time::Instant)>,
    pub held: Vec<Brush>,
    pub pointer: Option<[f32; 2]>,
    synth_mode: Mode,
    pub panel: Option<PaintPanel>,
    pub snap: bool,
    pub time_snap: bool,
    pub expression: haven::SliderState,
    pub strength: haven::SliderState,
    pub radius: haven::SliderState,
    pub selected: Option<usize>,
    pub low_pitch: f32,
    pub span: f32,
    pub offset: f32,
    gesture: Option<(Snapshot, Vec<[f32; 3]>, Brush)>,
}

impl Default for Painting {
    fn default() -> Self {
        Self {
            open: false,
            brush: haven::DropdownState {
                selected: Brush::Draw,
                hovered: None,
                expanded: false,
                depressed: false,
            },
            loop_menu: haven::DropdownState {
                selected: 1,
                ..Default::default()
            },
            last_tap: None,
            held: Vec::new(),
            pointer: None,
            synth_mode: Mode::Normal,
            panel: None,
            snap: false,
            time_snap: false,
            expression: haven::SliderState {
                value: 1.0,
                ..Default::default()
            },
            strength: haven::SliderState {
                value: 0.2,
                ..Default::default()
            },
            radius: haven::SliderState {
                value: 36.0,
                ..Default::default()
            },
            selected: None,
            low_pitch: 42.0,
            span: 1.0,
            offset: 0.0,
            gesture: None,
        }
    }
}

impl Painting {
    pub(crate) fn tool(&self) -> Brush {
        self.gesture
            .as_ref()
            .map(|(_, _, tool)| *tool)
            .or_else(|| self.held.last().copied())
            .unwrap_or(self.brush.selected)
    }

    pub(crate) fn preview(&self) -> &[[f32; 3]] {
        self.gesture
            .as_ref()
            .map(|(_, points, _)| points.as_slice())
            .unwrap_or(&[])
    }
}

pub(crate) fn stroke_bounds(stroke: &Stroke) -> [[f32; 2]; 2] {
    let mut low = [stroke.points()[0][0], 127.0_f32];
    let mut high = [stroke.points().last().unwrap()[0], 0.0_f32];
    for point in stroke.points() {
        low[1] = low[1].min(point[1]);
        high[1] = high[1].max(point[1]);
    }
    if high[1] - low[1] < 1.0 {
        let center = (low[1] + high[1]) * 0.5;
        low[1] = (center - 0.5).max(0.0);
        high[1] = (center + 0.5).min(127.0);
    }
    [low, high]
}

impl GuiState {
    pub(crate) fn transform_stroke(
        &mut self,
        phase: EnvPointerPhase,
        handle: [i8; 2],
        delta: [f32; 2],
    ) {
        let Some(index) = self.painting.selected else {
            return;
        };
        if phase == EnvPointerPhase::Start {
            self.painting.gesture = Some((self.snapshot(), Vec::new(), Brush::Select));
        }
        let Some((before, _, _)) = self.painting.gesture.as_ref() else {
            return;
        };
        let Some(stroke) = before.instrument.sequence.strokes().get(index) else {
            return;
        };
        let [low, high] = stroke_bounds(stroke);
        let mut points = stroke.points().to_vec();
        for axis in 0..2 {
            let limit = if axis == 0 {
                self.sequence().bars() as f32
            } else {
                127.0
            };
            if handle == [0, 0] {
                let shift = delta[axis].clamp(-low[axis], limit - high[axis]);
                for point in &mut points {
                    point[axis] += shift;
                }
            } else if handle[axis] != 0 {
                let mut end = [low[axis], high[axis]];
                let minimum = (high[axis] - low[axis]).min(0.0001);
                if handle[axis] < 0 {
                    end[0] = (end[0] + delta[axis]).clamp(0.0, end[1] - minimum);
                } else {
                    end[1] = (end[1] + delta[axis]).clamp(end[0] + minimum, limit);
                }
                for point in &mut points {
                    point[axis] = end[0]
                        + (point[axis] - low[axis]) / (high[axis] - low[axis]) * (end[1] - end[0]);
                }
            }
        }
        let mut strokes = self.sequence().strokes().to_vec();
        let sequence = Stroke::try_from(points).and_then(|stroke| {
            strokes[index] = stroke;
            Sequence::new(strokes, self.sequence().bars())
        });
        if phase == EnvPointerPhase::End {
            let (before, _, _) = self.painting.gesture.take().unwrap();
            self.commit_paint(before, sequence);
        } else {
            match sequence {
                Ok(sequence) => self.instrument.sequence = sequence,
                Err(error) => self.document_status = error.to_string(),
            }
        }
    }

    pub(crate) fn toggle_sequence(&mut self) {
        if !matches!(
            self.mode,
            Mode::Normal
                | Mode::Edit { .. }
                | Mode::AdsrEdit { .. }
                | Mode::EnvEdit { editing: false, .. }
                | Mode::ProbeEdit { .. }
                | Mode::SampleView { .. }
        ) {
            return;
        }
        if self.painting.open {
            if let Some((before, _, _)) = self.painting.gesture.take() {
                self.restore(before);
            }
            self.mode = match self.painting.synth_mode {
                Mode::Edit { module, .. }
                | Mode::AdsrEdit { module, .. }
                | Mode::EnvEdit { module, .. }
                | Mode::ProbeEdit { module }
                | Mode::SampleView { module, .. }
                    if !self
                        .modules()
                        .iter()
                        .any(|candidate| candidate.id == module) =>
                {
                    Mode::Normal
                }
                mode => mode,
            };
        } else {
            self.painting.synth_mode = self.mode;
            self.mode = Mode::Normal;
        }
        self.painting.open = !self.painting.open;
        self.painting.panel = None;
        self.painting.held.clear();
        self.painting.pointer = None;
    }

    pub(crate) fn sequence_playhead(&self) -> f32 {
        self.audio
            .as_ref()
            .map(AudioHandle::playhead)
            .unwrap_or(0.0)
    }

    pub(crate) fn sequence(&self) -> &Sequence {
        &self.instrument.sequence
    }

    pub(crate) fn sequence_pitch(&self, pitch: f32) -> f32 {
        if self.scale_index == 0 {
            return pitch.round().clamp(0.0, 127.0);
        }
        let scale = scale_from_index(self.scale_index);
        (-100..100)
            .map(|degree| scale.note(degree))
            .filter(|pitch| (0..=127).contains(pitch))
            .min_by(|a, b| ((*a as f32 - pitch).abs()).total_cmp(&(*b as f32 - pitch).abs()))
            .unwrap_or(60) as f32
    }

    pub(crate) fn paint_pointer(
        &mut self,
        phase: EnvPointerPhase,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        if phase == EnvPointerPhase::Start && self.painting.panel.take().is_some() {
            return;
        }
        let time = (self.painting.offset + x.clamp(0.0, 1.0) * self.painting.span)
            .min(self.sequence().bars() as f32);
        let pitch = (self.painting.low_pitch + (1.0 - y.clamp(0.0, 1.0)) * 36.0).clamp(0.0, 127.0);
        if phase == EnvPointerPhase::Start {
            let tool = self.painting.tool();
            if matches!(tool, Brush::Shape | Brush::Select) {
                let radius = if tool == Brush::Select {
                    8.0
                } else {
                    self.painting.radius.value
                };
                let hits = self
                    .sequence()
                    .strokes()
                    .iter()
                    .enumerate()
                    .filter_map(|(index, stroke)| {
                        let distance = stroke
                            .points()
                            .windows(2)
                            .map(|pair| {
                                let a = [
                                    (pair[0][0] - time) * width / self.painting.span,
                                    (pair[0][1] - pitch) * height / 36.0,
                                ];
                                let delta = [
                                    (pair[1][0] - pair[0][0]) * width / self.painting.span,
                                    (pair[1][1] - pair[0][1]) * height / 36.0,
                                ];
                                let fraction = (-(a[0] * delta[0] + a[1] * delta[1])
                                    / (delta[0] * delta[0] + delta[1] * delta[1])
                                        .max(f32::EPSILON))
                                .clamp(0.0, 1.0);
                                (a[0] + fraction * delta[0]).hypot(a[1] + fraction * delta[1])
                            })
                            .min_by(f32::total_cmp)
                            .unwrap();
                        (distance < radius).then_some((index, distance))
                    })
                    .collect::<Vec<_>>();
                self.painting.selected = hits
                    .iter()
                    .position(|(index, _)| Some(*index) == self.painting.selected)
                    .map(|hit| {
                        hits[if tool == Brush::Select {
                            (hit + 1) % hits.len()
                        } else {
                            hit
                        }]
                        .0
                    })
                    .or_else(|| {
                        hits.iter()
                            .min_by(|a, b| a.1.total_cmp(&b.1))
                            .map(|hit| hit.0)
                    });
            }
            let points = if tool == Brush::Draw {
                self.painting.selected = self
                    .sequence()
                    .strokes()
                    .iter()
                    .enumerate()
                    .filter_map(|(index, stroke)| {
                        let end = stroke.points().last().unwrap();
                        let distance = ((end[0] - time) * width / self.painting.span)
                            .hypot((end[1] - pitch) * height / 36.0);
                        (distance <= 10.0).then_some((index, distance))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|hit| hit.0);
                self.painting
                    .selected
                    .map(|index| self.sequence().strokes()[index].points().to_vec())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            self.painting.gesture = Some((self.snapshot(), points, tool));
        }
        if self.painting.gesture.is_none() {
            return;
        }
        if self.painting.tool() == Brush::Select {
            if phase == EnvPointerPhase::End {
                self.painting.gesture = None;
            }
            return;
        }
        if self.painting.tool() == Brush::Draw {
            let t = if self.painting.time_snap {
                (time * 16.0).round() / 16.0
            } else {
                time
            };
            let p = if self.painting.snap {
                self.sequence_pitch(pitch)
            } else {
                pitch
            };
            let expression = self
                .painting
                .selected
                .map(|index| self.sequence().strokes()[index].points().last().unwrap()[2])
                .unwrap_or(0.0);
            let extending_before_end = self.painting.selected.is_some_and(|index| {
                t <= self.sequence().strokes()[index].points().last().unwrap()[0]
            });
            let points = &mut self.painting.gesture.as_mut().unwrap().1;
            let index = points.partition_point(|point| point[0] < t);
            if !extending_before_end {
                if index < points.len() && (points[index][0] - t).abs() < 0.0001 {
                    points[index] = [t, p, expression];
                } else {
                    points.insert(index, [t, p, expression]);
                }
            }
            if phase != EnvPointerPhase::End {
                return;
            }
        }
        let mut strokes = self.sequence().strokes().to_vec();
        if self.painting.tool() != Brush::Draw {
            let radius = self.painting.radius.value;
            let span = self.painting.span;
            let brush = self.painting.tool();
            let strength = self.painting.strength.value;
            let expression = self.painting.expression.value;
            let mut erased = Vec::new();
            for (index, stroke) in strokes.iter_mut().enumerate() {
                if brush == Brush::Shape && self.painting.selected != Some(index) {
                    continue;
                }
                let mut points = Vec::new();
                for pair in stroke.points().windows(2) {
                    let steps = (((pair[1][0] - pair[0][0]) * width / span / 4.0).ceil() as usize)
                        .clamp(1, 4096);
                    for step in 0..steps {
                        let f = step as f32 / steps as f32;
                        points.push(std::array::from_fn(|axis| {
                            pair[0][axis] + (pair[1][axis] - pair[0][axis]) * f
                        }));
                    }
                }
                points.push(*stroke.points().last().unwrap());
                let mut touched = false;
                for point in &mut points {
                    let distance = (((point[0] - time) * width / span).powi(2)
                        + ((point[1] - pitch) * height / 36.0).powi(2))
                    .sqrt();
                    if distance >= radius {
                        continue;
                    }
                    touched = true;
                    let influence = strength * (1.0 - distance / radius);
                    match brush {
                        Brush::Shape => point[1] += (pitch - point[1]) * influence,
                        Brush::Quantize => {
                            point[1] += (self.sequence_pitch(point[1]) - point[1]) * influence
                        }
                        Brush::Expression => point[2] += (expression - point[2]) * influence,
                        Brush::Erase | Brush::Draw | Brush::Select => {}
                    }
                }
                if touched {
                    self.painting.selected = Some(index);
                    if brush == Brush::Erase {
                        erased.push(index);
                    } else if brush != Brush::Select {
                        *stroke = Stroke::try_from(points).unwrap();
                    }
                }
            }
            for index in erased.into_iter().rev() {
                strokes.remove(index);
                self.painting.selected = None;
            }
        }
        if phase == EnvPointerPhase::End {
            let (before, mut points, tool) = self.painting.gesture.take().unwrap();
            if tool == Brush::Draw {
                if points.len() == 1 {
                    let mut end = points[0];
                    end[0] = (end[0] + 1.0 / 64.0).min(self.sequence().bars() as f32);
                    if end[0] > points[0][0] {
                        points.push(end);
                    }
                }
                match Stroke::try_from(points) {
                    Ok(stroke) => {
                        if let Some(index) = self.painting.selected {
                            strokes[index] = stroke;
                        } else {
                            strokes.push(stroke);
                            self.painting.selected = Some(strokes.len() - 1);
                        }
                    }
                    Err(error) => {
                        self.commit_paint(before, Err(error));
                        return;
                    }
                }
            }
            self.commit_paint(before, Sequence::new(strokes, self.sequence().bars()));
        } else {
            match Sequence::new(strokes, self.sequence().bars()) {
                Ok(sequence) => self.instrument.sequence = sequence,
                Err(error) => {
                    let (before, _, _) = self.painting.gesture.take().unwrap();
                    self.restore(before);
                    self.document_status = error.to_string();
                }
            }
        }
    }

    fn commit_paint(&mut self, before: Snapshot, sequence: Result<Sequence, &'static str>) {
        match sequence {
            Ok(sequence) => self.instrument.sequence = sequence,
            Err(error) => {
                self.restore(before);
                self.document_status = error.to_string();
                return;
            }
        }
        self.painting.loop_menu.selected = self.sequence().bars();
        if before != self.snapshot() {
            self.undo.push(before);
            self.redo.clear();
            self.dirty = true;
            self.document_status = "Sequence edited".to_string();
            self.sync_audio_track();
        }
    }

    pub(crate) fn snap_paint(&mut self) {
        let before = self.snapshot();
        let mut strokes = self.sequence().strokes().to_vec();
        for (index, stroke) in strokes.iter_mut().enumerate() {
            if self
                .painting
                .selected
                .is_some_and(|selected| selected != index)
            {
                continue;
            }
            let mut points = stroke.points().to_vec();
            for point in &mut points {
                point[1] = self.sequence_pitch(point[1]);
            }
            if self.painting.time_snap {
                let start = points[0][0];
                let end = points.last().unwrap()[0];
                let snapped_start =
                    ((start * 16.0).round() / 16.0).min(self.sequence().bars() as f32 - 1.0 / 16.0);
                let snapped_end = ((end * 16.0).round() / 16.0)
                    .max(snapped_start + 1.0 / 16.0)
                    .min(self.sequence().bars() as f32);
                for point in &mut points {
                    point[0] = snapped_start
                        + (point[0] - start) / (end - start) * (snapped_end - snapped_start);
                }
            }
            *stroke = Stroke::try_from(points).unwrap();
        }
        self.commit_paint(before, Sequence::new(strokes, self.sequence().bars()));
    }

    pub(crate) fn resize_loop(&mut self, bars: u16) {
        let before = self.snapshot();
        self.commit_paint(
            before,
            Sequence::new(self.sequence().strokes().to_vec(), bars),
        );
        self.painting.span = self.painting.span.min(self.sequence().bars() as f32);
        self.painting.offset = self
            .painting
            .offset
            .min(self.sequence().bars() as f32 - self.painting.span);
    }

    pub(crate) fn painting_key(&mut self, action: GuiAction) -> bool {
        if self.painting.gesture.is_some() && action != GuiAction::Cancel {
            return true;
        }
        match action {
            GuiAction::Cancel => {
                if self.painting.brush.expanded || self.painting.loop_menu.expanded {
                    self.painting.brush.expanded = false;
                    self.painting.loop_menu.expanded = false;
                } else if let Some((before, _, _)) = self.painting.gesture.take() {
                    self.restore(before);
                } else if self.painting.panel.take().is_none() {
                    self.toggle_sequence();
                }
            }
            GuiAction::Delete => {
                if let Some(index) = self
                    .painting
                    .selected
                    .take()
                    .filter(|index| *index < self.sequence().strokes().len())
                {
                    let before = self.snapshot();
                    let mut strokes = self.sequence().strokes().to_vec();
                    strokes.remove(index);
                    self.commit_paint(before, Sequence::new(strokes, self.sequence().bars()));
                }
            }
            GuiAction::Left | GuiAction::Right | GuiAction::Up | GuiAction::Down => {
                if let Some(index) = self
                    .painting
                    .selected
                    .filter(|index| *index < self.sequence().strokes().len())
                {
                    let before = self.snapshot();
                    let mut points = self.sequence().strokes()[index].points().to_vec();
                    let dt = match action {
                        GuiAction::Left => -1.0 / 64.0,
                        GuiAction::Right => 1.0 / 64.0,
                        _ => 0.0,
                    };
                    let dp = match action {
                        GuiAction::Down => -0.1,
                        GuiAction::Up => 0.1,
                        _ => 0.0,
                    };
                    if points.iter().all(|point| {
                        point[0] + dt >= 0.0
                            && point[0] + dt <= self.sequence().bars() as f32
                            && (0.0..=127.0).contains(&(point[1] + dp))
                    }) {
                        for point in &mut points {
                            point[0] += dt;
                            point[1] += dp;
                        }
                        let mut strokes = self.sequence().strokes().to_vec();
                        strokes[index] = Stroke::try_from(points).unwrap();
                        self.commit_paint(before, Sequence::new(strokes, self.sequence().bars()));
                    }
                }
            }
            GuiAction::TogglePlay
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Quit
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit => return false,
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> GuiState {
        let mut state = GuiState::new(16, 16);
        state.instrument.sequence = Sequence::new(Vec::new(), 1).unwrap();
        state.apply(GuiAction::TrackEdit);
        state
    }

    fn drag(state: &mut GuiState, from: [f32; 2], to: [f32; 2]) {
        state.paint_pointer(EnvPointerPhase::Start, from[0], from[1], 800.0, 360.0);
        state.paint_pointer(EnvPointerPhase::End, to[0], to[1], 800.0, 360.0);
    }

    #[test]
    fn held_tools_restore_in_order_and_keep_the_gesture_tool_until_release() {
        use haven::{Key, MouseButton, PaneBuilder, Point};
        let mut state = canvas();
        drag(&mut state, [0.1, 0.5], [0.9, 0.5]);
        let mut pane = PaneBuilder::new("main", crate::view::main_view).build();
        pane.redraw(&mut state, 1060, 760, 1.0);
        pane.key_pressed(&mut state, Key::character("q"));
        pane.key_pressed(&mut state, Key::character("q"));
        assert_eq!(state.painting.held.len(), 1);
        assert_eq!(state.painting.tool(), Brush::Quantize);
        pane.key_pressed(&mut state, Key::character("v"));
        assert_eq!(state.painting.tool(), Brush::Shape);
        pane.key_released(&mut state, Key::character("v"));
        assert_eq!(state.painting.tool(), Brush::Quantize);
        let center = pane.location(71_901).unwrap();
        pane.move_to(&mut state, center);
        pane.press_button(&mut state, MouseButton::Left);
        pane.move_to(&mut state, Point::new(center.x + 15., center.y));
        pane.key_released(&mut state, Key::character("q"));
        assert_eq!(state.painting.tool(), Brush::Quantize);
        pane.release_button(&mut state, MouseButton::Left);
        assert_eq!(state.painting.tool(), Brush::Draw);
        assert_eq!(state.sequence().strokes().len(), 1);
        pane.key_pressed(&mut state, Key::character("]"));
        assert_eq!(state.painting.radius.value, 45.);
        pane.key_pressed(&mut state, Key::character("q"));
        pane.key_released(&mut state, Key::character("Q"));
        assert_eq!(state.painting.tool(), Brush::Draw);
    }

    #[test]
    fn brush_cursor_tracks_drag_and_tab_restores_synth_editor() {
        use haven::{Key, MouseButton, NamedKey, PaneBuilder, Point};
        let mut state = canvas();
        let mut pane = PaneBuilder::new("main", crate::view::main_view).build();
        pane.redraw(&mut state, 1060, 760, 1.0);
        let center = pane.location(71_901).unwrap();
        pane.move_to(&mut state, center);
        pane.key_pressed(&mut state, Key::character("r"));
        pane.press_button(&mut state, MouseButton::Left);
        pane.move_to(&mut state, Point::new(center.x + 20.0, center.y));
        pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(pane.location(71_905).is_some());
        pane.release_button(&mut state, MouseButton::Left);
        assert!(state.painting.pointer.is_none());
        assert_eq!(state.painting.tool(), Brush::Erase);
        pane.move_to(&mut state, Point::new(2., 2.));
        assert!(state.painting.pointer.is_none());
        pane.key_released(&mut state, Key::character("r"));
        pane.key_pressed(&mut state, NamedKey::Tab);
        assert!(!state.painting.open);
        state.instrument.root.modules.push(Module {
            id: ModuleId::new(2),
            position: GridPos::new(0, 0),
            orientation: Orientation::Right,
            body: ModuleKind::Probe.default_body(),
            disabled: false,
        });
        state.mode = Mode::ProbeEdit {
            module: ModuleId::new(2),
        };
        pane.key_pressed(&mut state, NamedKey::Tab);
        assert!(state.painting.open);
        assert_eq!(state.mode, Mode::Normal);
        pane.key_pressed(&mut state, NamedKey::Tab);
        assert_eq!(
            state.mode,
            Mode::ProbeEdit {
                module: ModuleId::new(2)
            }
        );
        assert!(!state.painting.open);
    }

    #[test]
    fn double_tap_locks_a_brush_and_held_settings_survive_repeat_and_release() {
        use haven::{Key, PaneBuilder, Point};
        let mut state = canvas();
        let mut pane = PaneBuilder::new("main", crate::view::main_view).build();
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        for _ in 0..2 {
            assert!(pane.key_pressed(&mut state, Key::character("r")).is_empty());
            assert!(
                pane.key_released(&mut state, Key::character("r"))
                    .is_empty()
            );
        }
        assert_eq!(state.painting.tool(), Brush::Erase);
        assert!(pane.key_pressed(&mut state, Key::character("q")).is_empty());
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        assert!(
            pane.click(&mut state, pane.location(70_124).unwrap())
                .is_empty()
        );
        assert!(pane.key_pressed(&mut state, Key::character("q")).is_empty());
        assert!(
            pane.key_released(&mut state, Key::character("q"))
                .is_empty()
        );
        assert_eq!(
            state.painting.panel,
            Some(PaintPanel::Options(Brush::Quantize))
        );
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        let slider = pane.location(72_020).unwrap();
        assert!(
            pane.drag(
                &mut state,
                Point::new(slider.x - 60.0, slider.y),
                Point::new(slider.x + 60.0, slider.y)
            )
            .is_empty()
        );
        assert!(state.painting.radius.value > 60.0);
        assert_eq!(state.painting.tool(), Brush::Erase);
    }

    #[test]
    fn brush_size_is_visible_and_loop_uses_native_selection() {
        use haven::{Key, PaneBuilder, Point, render::RenderItem};
        let mut state = canvas();
        let mut pane = PaneBuilder::new("main", crate::view::main_view).build();
        state.painting.brush.selected = Brush::Erase;
        for resize in [false, true] {
            if resize {
                assert!(pane.key_pressed(&mut state, Key::character("]")).is_empty());
            }
            let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
            assert!(effects.is_empty());
            let center = pane.location(71_905).unwrap();
            assert!(frame.items.iter().any(|item| match item {
                RenderItem::Path { area, .. } =>
                    area.width == state.painting.radius.value * 2.0
                        && area.height == area.width
                        && (f64::from(area.x + area.width / 2.0) - center.x).abs() < 1.0,
                _ => false,
            }));
        }
        assert_eq!(state.painting.radius.value, 45.0);
        assert!(
            pane.click(&mut state, pane.location(70_124).unwrap())
                .is_empty()
        );
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        assert!(pane.click(&mut state, Point::new(1000.0, 700.0)).is_empty());
        assert!(state.painting.panel.is_none());
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        assert!(
            pane.click(&mut state, pane.location(74_100).unwrap())
                .is_empty()
        );
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        assert!(
            pane.click(&mut state, pane.location(74_101).unwrap())
                .is_empty()
        );
        assert_eq!(state.sequence().bars(), 2);
        assert!(!state.painting.loop_menu.expanded);
        state.apply(GuiAction::Undo);
        assert_eq!(state.sequence().bars(), 1);
        assert_eq!(state.painting.loop_menu.selected, 1);
    }

    #[test]
    fn disclosure_is_contextual_and_does_not_move_or_edit_the_canvas() {
        use haven::{Key, NamedKey, PaneBuilder, Point};
        let mut state = canvas();
        let mut pane = PaneBuilder::new("main", crate::view::main_view).build();
        pane.redraw(&mut state, 1060, 760, 1.0);
        let canvas_center = pane.location(71_901).unwrap();
        assert!(canvas_center.y < 470.0, "{canvas_center:?}");
        assert!(pane.location(70_000).is_none());
        assert!(pane.location(72_000).is_none());
        pane.click(&mut state, pane.location(74_000).unwrap());
        pane.redraw(&mut state, 1060, 760, 1.0);
        assert_eq!(pane.location(71_901).unwrap(), canvas_center);
        pane.click(&mut state, pane.location(74_004).unwrap());
        assert_eq!(state.painting.brush.selected, Brush::Expression);
        assert_eq!(state.painting.panel, None);
        pane.redraw(&mut state, 1060, 760, 1.0);
        pane.click(&mut state, pane.location(70_124).unwrap());
        pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(pane.location(70_030).is_none());
        let slider = pane.location(72_000).unwrap();
        pane.click(&mut state, Point::new(slider.x - 70.0, slider.y));
        assert!(state.painting.expression.value < -0.4);
        assert_eq!(
            state.painting.panel,
            Some(PaintPanel::Options(Brush::Expression))
        );
        assert!(state.sequence().strokes().is_empty());
        pane.key_pressed(&mut state, NamedKey::Escape);
        assert_eq!(state.painting.panel, None);
        assert!(state.painting.open);
        pane.key_pressed(&mut state, Key::character("b"));
        pane.redraw(&mut state, 1060, 760, 1.0);
        pane.click(&mut state, pane.location(70_124).unwrap());
        pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(pane.location(70_030).is_some());
        assert!(pane.location(72_000).is_none());
        pane.click(&mut state, canvas_center);
        assert_eq!(state.painting.panel, None);
        assert!(state.sequence().strokes().is_empty());
        pane.key_pressed(&mut state, NamedKey::Escape);
        assert!(!state.painting.open);
    }

    #[test]
    fn selecting_preserves_the_document_and_six_voice_limit_is_explicit() {
        let mut state = canvas();
        drag(&mut state, [0.1, 0.5], [0.9, 0.5]);
        let before = state.sequence().clone();
        let undo = state.undo.len();
        state.painting.brush.selected = Brush::Select;
        drag(&mut state, [0.5, 0.5], [0.51, 0.5]);
        assert_eq!(state.sequence(), &before);
        assert_eq!(state.undo.len(), undo);
        assert_eq!(state.painting.selected, Some(0));
        state.painting.brush.selected = Brush::Draw;
        for _ in 0..6 {
            drag(&mut state, [0.1, 0.5], [0.9, 0.5]);
        }
        assert_eq!(state.sequence().strokes().len(), 6);
        assert_eq!(
            state.document_status,
            "This instrument supports six simultaneous strokes"
        );
    }

    #[test]
    fn overlapping_strokes_can_be_selected_and_shaped_independently() {
        let mut state = canvas();
        drag(&mut state, [0.1, 0.5], [0.9, 0.5]);
        drag(&mut state, [0.1, 0.5], [0.9, 0.5]);
        let original = state.sequence().clone();
        state.painting.brush.selected = Brush::Select;
        drag(&mut state, [0.5, 0.5], [0.5, 0.5]);
        assert_eq!(state.painting.selected, Some(0));
        drag(&mut state, [0.5, 0.5], [0.5, 0.5]);
        assert_eq!(state.painting.selected, Some(1));
        assert_eq!(state.sequence(), &original);
        let undo = state.undo.len();
        state.painting.brush.selected = Brush::Shape;
        drag(&mut state, [0.5, 0.5], [0.5, 0.45]);
        assert_eq!(state.sequence().strokes()[0], original.strokes()[0]);
        assert_ne!(state.sequence().strokes()[1], original.strokes()[1]);
        assert_eq!(state.painting.selected, Some(1));
        assert_eq!(state.undo.len(), undo + 1);
        state.apply(GuiAction::Undo);
        assert_eq!(state.sequence(), &original);
    }

    #[test]
    fn drawing_near_an_endpoint_extends_the_stroke_with_one_undo() {
        let mut state = canvas();
        drag(&mut state, [0.1, 0.5], [0.5, 0.5]);
        state.instrument.sequence = Sequence::new(
            vec![Stroke::try_from(vec![[0.1, 60.0, 0.75], [0.5, 60.0, 0.75]]).unwrap()],
            1,
        )
        .unwrap();
        let original = state.sequence().clone();
        let undo = state.undo.len();
        drag(&mut state, [0.505, 0.5], [0.8, 0.4]);
        assert_eq!(state.sequence().strokes().len(), 1);
        assert!(
            state.sequence().strokes()[0]
                .points()
                .starts_with(original.strokes()[0].points())
        );
        assert_eq!(
            state.sequence().strokes()[0].points().last().unwrap()[0],
            0.8
        );
        assert_eq!(
            state.sequence().strokes()[0].points().last().unwrap()[2],
            0.75
        );
        assert_eq!(state.undo.len(), undo + 1);
        state.apply(GuiAction::Undo);
        assert_eq!(state.sequence(), &original);
    }

    #[test]
    fn default_expression_brush_changes_a_new_stroke() {
        let mut state = canvas();
        drag(&mut state, [0.1, 0.5], [0.9, 0.5]);
        assert!(
            state.sequence().strokes()[0]
                .points()
                .iter()
                .all(|point| point[2] == 0.0)
        );
        state.painting.brush.selected = Brush::Expression;
        drag(&mut state, [0.49, 0.5], [0.51, 0.5]);
        assert!(
            state.sequence().strokes()[0]
                .points()
                .iter()
                .any(|point| point[2] > 0.0)
        );
    }

    #[test]
    fn freehand_stroke_remains_fractional_and_undo_is_one_gesture() {
        let mut state = canvas();
        drag(&mut state, [0.123, 0.333], [0.777, 0.111]);
        let painted = state.sequence().clone();
        assert_eq!(painted.strokes().len(), 1);
        assert_eq!(painted.strokes()[0].points()[0][0], 0.123);
        assert_ne!(painted.strokes()[0].points()[0][1].fract(), 0.0);
        state.apply(GuiAction::Undo);
        assert!(state.sequence().strokes().is_empty());
        state.apply(GuiAction::Redo);
        assert_eq!(state.sequence(), &painted);
    }

    #[test]
    fn quantization_brush_gently_moves_only_nearby_pitch() {
        let mut state = canvas();
        drag(&mut state, [0.1, 0.337], [0.9, 0.337]);
        let original = state.sequence().strokes()[0].points()[0][1];
        let target = state.sequence_pitch(original);
        state.painting.brush.selected = Brush::Quantize;
        drag(&mut state, [0.49, 0.337], [0.51, 0.337]);
        let stroke = &state.sequence().strokes()[0];
        let changed = stroke
            .points()
            .iter()
            .min_by(|a, b| (a[0] - 0.5).abs().total_cmp(&(b[0] - 0.5).abs()))
            .unwrap()[1];
        assert!((changed - target).abs() < (original - target).abs());
        assert_ne!(changed, target);
        assert_eq!(stroke.points()[0][1], original);
    }

    #[test]
    fn expression_brush_paints_the_middle_without_changing_pitch_or_ends() {
        let mut state = canvas();
        drag(&mut state, [0.1, 0.5], [0.9, 0.5]);
        state.painting.brush.selected = Brush::Expression;
        state.painting.expression.value = -1.0;
        state.painting.strength.value = 1.0;
        drag(&mut state, [0.49, 0.5], [0.51, 0.5]);
        let stroke = &state.sequence().strokes()[0];
        assert_eq!(&stroke.points()[0][1..], &[60.0, 0.0]);
        assert!(stroke.points().iter().any(|point| point[2] < 0.0));
        assert_eq!(
            stroke
                .points()
                .iter()
                .min_by(|a, b| (a[0] - 0.5).abs().total_cmp(&(b[0] - 0.5).abs()))
                .unwrap()[1],
            60.0
        );
        assert_eq!(&stroke.points().last().unwrap()[1..], &[60.0, 0.0]);
    }

    #[test]
    fn snapping_drawing_and_editing_are_optional() {
        let mut state = canvas();
        state.painting.snap = true;
        state.painting.time_snap = true;
        drag(&mut state, [0.13, 0.337], [0.78, 0.321]);
        for point in state.sequence().strokes()[0].points() {
            assert_eq!((point[0] * 16.0).fract(), 0.0);
            assert_eq!(point[1].fract(), 0.0);
        }
        state.painting.snap = false;
        state.painting.time_snap = false;
        drag(&mut state, [0.13, 0.337], [0.78, 0.321]);
        assert_ne!(state.sequence().strokes()[1].points()[0][1].fract(), 0.0);
        state.snap_paint();
        assert_eq!(state.sequence().strokes()[1].points()[0][1].fract(), 0.0);
    }

    #[test]
    fn painted_project_saves_loads_and_replaces_legacy_input() {
        let mut state = canvas();
        drag(&mut state, [0.13, 0.337], [0.78, 0.321]);
        state.painting.brush.selected = Brush::Expression;
        state.painting.expression.value = -0.8;
        drag(&mut state, [0.5, 0.33], [0.52, 0.33]);
        state.instrument.root.modules.push(Module {
            id: ModuleId::new(100),
            position: GridPos::new(0, 0),
            orientation: Orientation::Right,
            body: ModuleKind::Expression.default_body(),
            disabled: false,
        });
        let sequence = state.sequence().clone();
        let project = state.project().unwrap();
        let path =
            std::env::temp_dir().join(format!("brainwash-expression-{}.bw", std::process::id()));
        brainwash_grid::project::save(&path, &project).unwrap();
        let project = brainwash_grid::project::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        let mut restored = canvas();
        restored.set_project(project, "painted.bw".into()).unwrap();
        assert_eq!(restored.sequence(), &sequence);
        assert!(!restored.dirty());
    }

    #[test]
    fn selected_stroke_transforms_preserve_expression_and_undo() {
        let mut state = canvas();
        drag(&mut state, [0.2, 0.6], [0.5, 0.3]);
        let original = state.sequence().clone();
        state.painting.brush.selected = Brush::Select;
        state.transform_stroke(EnvPointerPhase::Start, [0, 0], [0.0, 0.0]);
        state.transform_stroke(EnvPointerPhase::Drag, [0, 0], [0.1, 2.0]);
        let moved = state.sequence().clone();
        state.transform_stroke(EnvPointerPhase::End, [0, 0], [0.1, 2.0]);
        assert_eq!(state.sequence(), &moved);
        for (before, after) in original.strokes()[0]
            .points()
            .iter()
            .zip(moved.strokes()[0].points())
        {
            assert!((after[0] - before[0] - 0.1).abs() < 0.00001);
            assert_eq!(after[1], before[1] + 2.0);
            assert_eq!(after[2], before[2]);
        }
        state.apply(GuiAction::Undo);
        assert_eq!(state.sequence(), &original);
        state.painting.selected = Some(0);
        state.transform_stroke(EnvPointerPhase::Start, [1, 1], [0.0, 0.0]);
        state.transform_stroke(EnvPointerPhase::End, [1, 1], [0.2, 3.0]);
        let [low, high] = stroke_bounds(&state.sequence().strokes()[0]);
        let [before_low, before_high] = stroke_bounds(&original.strokes()[0]);
        assert_eq!(low, before_low);
        assert!((high[0] - before_high[0] - 0.2).abs() < 0.00001);
        assert_eq!(high[1], before_high[1] + 3.0);
        state.apply(GuiAction::Undo);
        assert_eq!(state.sequence(), &original);
        state.painting.selected = Some(0);
        state.transform_stroke(EnvPointerPhase::Start, [0, 0], [0.0, 0.0]);
        state.transform_stroke(EnvPointerPhase::Drag, [0, 0], [100.0, 1000.0]);
        let [_, high] = stroke_bounds(&state.sequence().strokes()[0]);
        assert!(high[0] <= 1.0 && high[1] <= 127.0);
        state.apply(GuiAction::Cancel);
        assert_eq!(state.sequence(), &original);
        for (start, end, handle) in [(0.0, 0.00005, [-1, 0]), (0.99995, 1.0, [1, 0])] {
            state.instrument.sequence = Sequence::new(
                vec![Stroke::try_from(vec![[start, 60.0, 0.0], [end, 60.0, 0.0]]).unwrap()],
                1,
            )
            .unwrap();
            state.painting.selected = Some(0);
            state.transform_stroke(EnvPointerPhase::Start, handle, [0.0, 0.0]);
            state.transform_stroke(EnvPointerPhase::End, handle, [1.0, 0.0]);
            assert_eq!(state.sequence().strokes().len(), 1);
        }
    }

    #[test]
    fn native_selection_box_moves_and_scales_a_stroke() {
        use haven::{PaneBuilder, Point};
        let mut state = canvas();
        drag(&mut state, [0.2, 0.6], [0.5, 0.3]);
        state.painting.brush.selected = Brush::Select;
        let mut pane = PaneBuilder::new("main", crate::view::main_view).build();
        let original = state.sequence().clone();
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        let center = pane.location(75_000).unwrap();
        assert!(
            pane.drag(
                &mut state,
                center,
                Point::new(center.x + 40.0, center.y + 20.0)
            )
            .is_empty()
        );
        let moved = state.sequence().clone();
        assert_ne!(moved, original);
        assert_eq!(moved.strokes().len(), 1);
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        let corner = pane.location(75_009).unwrap();
        assert!(
            pane.drag(
                &mut state,
                corner,
                Point::new(corner.x + 40.0, corner.y - 20.0)
            )
            .is_empty()
        );
        let [low, high] = stroke_bounds(&state.sequence().strokes()[0]);
        let [before_low, before_high] = stroke_bounds(&moved.strokes()[0]);
        assert_eq!(low, before_low);
        assert!(high[0] > before_high[0] && high[1] > before_high[1]);
    }

    #[test]
    fn native_selection_cycles_overlaps_before_shaping() {
        use haven::{PaneBuilder, Point};
        let mut state = canvas();
        drag(&mut state, [0.1, 0.5], [0.9, 0.5]);
        drag(&mut state, [0.1, 0.5], [0.9, 0.5]);
        let original = state.sequence().clone();
        let mut pane = PaneBuilder::new("main", crate::view::main_view).build();
        state.painting.brush.selected = Brush::Select;
        for expected in [0, 1, 0] {
            let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
            assert!(!frame.items.is_empty());
            assert!(effects.is_empty());
            let center = pane.location(71_901).unwrap();
            assert!(pane.click(&mut state, center).is_empty());
            assert_eq!(state.painting.selected, Some(expected));
        }
        state.painting.brush.selected = Brush::Shape;
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        let center = pane.location(71_901).unwrap();
        assert!(
            pane.drag(&mut state, center, Point::new(center.x, center.y - 20.0))
                .is_empty()
        );
        assert_ne!(state.sequence().strokes()[0], original.strokes()[0]);
        assert_eq!(state.sequence().strokes()[1], original.strokes()[1]);
    }

    #[test]
    fn native_pointer_events_draw_and_brush_the_canvas() {
        use haven::{PaneBuilder, Point};
        let mut state = canvas();
        let mut pane = PaneBuilder::new("main", crate::view::main_view).build();
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        let center = pane.location(71_901).unwrap();
        let effects = pane.drag(
            &mut state,
            Point::new(center.x - 200.0, center.y),
            Point::new(center.x + 200.0, center.y - 20.0),
        );
        assert!(effects.is_empty());
        assert_eq!(state.sequence().strokes().len(), 1);
        let initial = state.sequence().strokes()[0].points().to_vec();
        assert!(
            pane.drag(
                &mut state,
                Point::new(center.x + 205.0, center.y - 20.0),
                Point::new(center.x + 280.0, center.y - 50.0),
            )
            .is_empty()
        );
        assert_eq!(state.sequence().strokes().len(), 1);
        assert!(state.sequence().strokes()[0].points().starts_with(&initial));
        let original = state.sequence().clone();
        state.painting.brush.selected = Brush::Expression;
        state.painting.expression.value = -1.0;
        let (frame, effects) = pane.redraw(&mut state, 1060, 760, 1.0);
        assert!(!frame.items.is_empty());
        assert!(effects.is_empty());
        let effects = pane.drag(
            &mut state,
            Point::new(center.x - 10.0, center.y - 10.0),
            Point::new(center.x + 10.0, center.y - 10.0),
        );
        assert!(effects.is_empty());
        assert_ne!(state.sequence(), &original);
    }
}
