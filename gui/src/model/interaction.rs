use super::*;

impl GuiState {
    pub fn apply(&mut self, action: GuiAction) {
        let playing = self.playing;
        let bpm = self.bpm;
        let scale_index = self.scale_index;
        match self.mode {
            Mode::Normal => self.apply_normal(action),
            Mode::QuitConfirm => self.apply_quit_confirm(action),
            Mode::Palette => self.apply_palette(action),
            Mode::Move { module, origin } => self.apply_move(action, module, origin),
            Mode::Copy { source } => self.apply_copy(action, source),
            Mode::Edit { module, parameter } => self.apply_edit(action, module, parameter),
            Mode::ValueInput { module, parameter } => {
                self.apply_value_input(action, module, parameter);
            }
            Mode::AdsrEdit { module, parameter } => {
                self.apply_adsr_edit(action, module, parameter);
            }
            Mode::EnvEdit {
                module,
                point,
                editing,
            } => self.apply_env_edit(action, module, point, editing),
            Mode::ProbeEdit { module } => self.apply_probe_edit(action, module),
            Mode::SampleView {
                module,
                zoom,
                offset,
            } => self.apply_sample_view(action, module, zoom, offset),
            Mode::Select { anchor } => self.apply_select(action, anchor),
            Mode::SelectMove {
                anchor,
                extent,
                origin,
            } => self.apply_select_move(action, anchor, extent, origin),
            Mode::CopySelection {
                anchor,
                extent,
                origin,
            } => self.apply_copy_selection(action, anchor, extent, origin),
            Mode::LoadConfirm => self.apply_load_confirm(action),
            Mode::SaveConfirm => self.apply_save_confirm(action),
            Mode::ExportPrompt => self.apply_export_prompt(action),
            Mode::ExportConfirm => self.apply_export_confirm(action),
            Mode::TrackPrompt => self.apply_track_prompt(action),
            Mode::TrackSettings { parameter } => self.apply_track_settings(action, parameter),
        }
        if self.playing != playing {
            self.sync_audio_playing();
        }
        if self.bpm != bpm || self.scale_index != scale_index {
            self.sync_audio_track();
        }
        self.update_grid_view();
        self.collect_audio_retired();
    }

    pub(crate) fn focus_cell(&mut self, position: GridPos) {
        let x = position.x.min(self.grid_size.width().saturating_sub(1));
        let y = position.y.min(self.grid_size.height().saturating_sub(1));
        self.instrument_mut().surface_mut().cursor = GridPos::new(x, y);
        self.update_grid_view();
    }

    pub(crate) fn open_category(&mut self, category: ModuleCategory) {
        self.mode = Mode::Palette;
        self.palette_searching = false;
        self.palette_filter.clear();
        self.palette_filter_index = 0;
        self.palette_category = category;
        self.palette_index = 0;
    }

    pub(crate) fn choose_palette_index(&mut self, index: usize) {
        self.mode = Mode::Palette;
        self.palette_searching = false;
        self.palette_filter.clear();
        self.palette_filter_index = 0;
        self.palette_index = index.min(self.palette_modules().len().saturating_sub(1));
    }

    pub(crate) fn choose_filtered_palette_index(&mut self, index: usize) {
        self.mode = Mode::Palette;
        self.palette_searching = true;
        self.palette_filter_index =
            index.min(self.filtered_palette_modules().len().saturating_sub(1));
    }

    pub(crate) fn click_grid_cell(&mut self, position: GridPos) {
        let cursor = self.cursor();
        self.focus_cell(position);
        match self.mode {
            Mode::Palette => self.apply(GuiAction::Confirm),
            Mode::Select { anchor } => {
                let (anchor, extent) = normalized_rect(anchor, cursor);
                if position.x >= anchor.x
                    && position.x <= extent.x
                    && position.y >= anchor.y
                    && position.y <= extent.y
                {
                    self.mode = Mode::SelectMove {
                        anchor,
                        extent,
                        origin: position,
                    };
                } else {
                    self.mode = Mode::Normal;
                }
            }
            Mode::SelectMove { anchor, extent, .. } => {
                if position.x >= anchor.x
                    && position.x <= extent.x
                    && position.y >= anchor.y
                    && position.y <= extent.y
                {
                    self.mode = Mode::SelectMove {
                        anchor,
                        extent,
                        origin: position,
                    };
                } else {
                    self.mode = Mode::Normal;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn drag_grid_cell(&mut self, phase: GridPointerPhase, position: GridPos) {
        match phase {
            GridPointerPhase::Start => self.start_grid_drag(position),
            GridPointerPhase::Drag => self.update_grid_drag(position),
            GridPointerPhase::End => self.end_grid_drag(position),
        }
    }

    pub(crate) fn drag_env_point(
        &mut self,
        phase: EnvPointerPhase,
        module: ModuleId,
        time: f32,
        value: f32,
    ) {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            self.mode = Mode::Normal;
            return;
        };
        let point_count = self.modules()[module_index].env_points().len();
        if point_count == 0 {
            self.mode = Mode::Normal;
            return;
        }
        let target_time = (time * 100.).round().clamp(0., 100.) as i32;
        let target_value = (value * 100.).round().clamp(-100., 100.) as i32;
        let point = match (phase, self.mode) {
            (
                EnvPointerPhase::Drag | EnvPointerPhase::End,
                Mode::EnvEdit {
                    module: active,
                    point,
                    editing: true,
                },
            ) if active == module => point.min(point_count - 1),
            _ => self.modules()[module_index]
                .env_points()
                .iter()
                .enumerate()
                .min_by_key(|(_, point)| {
                    let time_delta = point.time - target_time;
                    let value_delta = point.value - target_value;
                    time_delta * time_delta + value_delta * value_delta
                })
                .map(|(index, _)| index)
                .unwrap_or(0),
        };
        let before = self.snapshot();
        let (point, changed) = {
            let Some(points) =
                self.instrument_mut().surface_mut().modules[module_index].env_points_mut()
            else {
                self.mode = Mode::Normal;
                return;
            };
            let changed = points[point].time != target_time || points[point].value != target_value;
            points[point].time = target_time;
            points[point].value = target_value;
            let moved = points[point];
            points.sort_by_key(|point| point.time);
            let point = points.iter().position(|point| *point == moved).unwrap_or(0);
            (point, changed)
        };
        if changed {
            self.commit(before);
        }
        self.mode = Mode::EnvEdit {
            module,
            point,
            editing: phase != EnvPointerPhase::End,
        };
    }

    pub(super) fn start_grid_drag(&mut self, position: GridPos) {
        let cursor = self.cursor();
        self.focus_cell(position);
        self.pointer_drag = match self.mode {
            Mode::Select { anchor } => {
                let (anchor, extent) = normalized_rect(anchor, cursor);
                if position.x >= anchor.x
                    && position.x <= extent.x
                    && position.y >= anchor.y
                    && position.y <= extent.y
                {
                    self.mode = Mode::SelectMove {
                        anchor,
                        extent,
                        origin: position,
                    };
                    Some(PointerDrag::SelectionMove {
                        anchor,
                        extent,
                        origin: position,
                    })
                } else {
                    self.mode = Mode::Normal;
                    None
                }
            }
            Mode::Normal => {
                let module_drag = self.module_at(position).map(|module| PointerDrag::Module {
                    module: module.id,
                    origin: module.position,
                    grab: position,
                });
                module_drag.or(Some(PointerDrag::Selection { anchor: position }))
            }
            _ => None,
        };
    }

    pub(super) fn update_grid_drag(&mut self, position: GridPos) {
        let Some(drag) = self.pointer_drag else {
            return;
        };
        match drag {
            PointerDrag::Module {
                module,
                origin,
                grab,
            } => {
                if position == self.cursor() {
                    return;
                }
                self.focus_cell(position);
                let Some(found) = self.modules().iter().find(|found| found.id == module) else {
                    return;
                };
                let next = self.moved_position(found, origin, grab, position);
                let before = self.snapshot();
                if let Some(found) = self
                    .instrument_mut()
                    .surface_mut()
                    .modules
                    .iter_mut()
                    .find(|found| found.id == module)
                {
                    found.position = next;
                    self.pointer_drag = Some(PointerDrag::Module {
                        module,
                        origin,
                        grab,
                    });
                    self.commit(before);
                }
            }
            PointerDrag::Selection { anchor } => {
                self.focus_cell(position);
                if position != anchor {
                    self.mode = Mode::Select { anchor };
                }
            }
            PointerDrag::SelectionMove {
                anchor,
                extent,
                origin,
            } => {
                if position == origin {
                    return;
                }
                let Some((anchor, extent)) =
                    self.move_selection_rect(anchor, extent, origin, position)
                else {
                    return;
                };
                self.focus_cell(position);
                self.mode = Mode::SelectMove {
                    anchor,
                    extent,
                    origin: position,
                };
                self.pointer_drag = Some(PointerDrag::SelectionMove {
                    anchor,
                    extent,
                    origin: position,
                });
            }
        }
    }

    fn end_grid_drag(&mut self, position: GridPos) {
        self.update_grid_drag(position);
        match self.pointer_drag {
            Some(PointerDrag::Selection { anchor }) if anchor == self.cursor() => {
                self.mode = Mode::Normal;
            }
            Some(PointerDrag::SelectionMove { anchor, extent, .. }) => {
                self.focus_cell(extent);
                self.mode = Mode::Select { anchor };
            }
            _ => {}
        }
        self.pointer_drag = None;
    }

    fn move_selection_rect(
        &mut self,
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
        target: GridPos,
    ) -> Option<(GridPos, GridPos)> {
        let ids = self
            .modules()
            .iter()
            .filter(|module| {
                module.position.x >= anchor.x
                    && module.position.x <= extent.x
                    && module.position.y >= anchor.y
                    && module.position.y <= extent.y
            })
            .map(|module| module.id)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return None;
        }
        let dx = target.x as i16 - origin.x as i16;
        let dy = target.y as i16 - origin.y as i16;
        let targets: Vec<(ModuleId, GridPos)> = self
            .modules()
            .iter()
            .filter(|module| ids.contains(&module.id))
            .map(|module| {
                let x = (module.position.x as i16 + dx)
                    .clamp(0, self.grid_size.width().saturating_sub(1) as i16);
                let y = (module.position.y as i16 + dy)
                    .clamp(0, self.grid_size.height().saturating_sub(1) as i16);
                (module.id, GridPos::new(x as u16, y as u16))
            })
            .collect();
        if targets.iter().any(|(id, position)| {
            let Some(module) = self.modules().iter().find(|module| module.id == *id) else {
                return true;
            };
            !self.module_fits(module, *position, &ids)
        }) {
            return None;
        }
        let before = self.snapshot();
        for (id, position) in targets {
            if let Some(module) = self
                .instrument_mut()
                .surface_mut()
                .modules
                .iter_mut()
                .find(|module| module.id == id)
            {
                module.position = position;
            }
        }
        self.commit(before);
        let x = (anchor.x as i16 + dx).clamp(0, self.grid_size.width().saturating_sub(1) as i16);
        let y = (anchor.y as i16 + dy).clamp(0, self.grid_size.height().saturating_sub(1) as i16);
        let end_x =
            (extent.x as i16 + dx).clamp(0, self.grid_size.width().saturating_sub(1) as i16);
        let end_y =
            (extent.y as i16 + dy).clamp(0, self.grid_size.height().saturating_sub(1) as i16);
        Some((
            GridPos::new(x as u16, y as u16),
            GridPos::new(end_x as u16, end_y as u16),
        ))
    }

    fn open_prompt(&mut self, mode: Mode, text: impl Into<String>) {
        self.prompt_text = text.into();
        self.prompt_cursor = self.prompt_text.len();
        self.prompt_input = TextState::new(&self.prompt_text);
        self.mode = mode;
    }

    fn apply_text_action(&mut self, action: GuiAction, kind: TextInputKind) {
        match action {
            GuiAction::InputChar(character) => {
                if text_input_accepts(kind, character) {
                    self.prompt_text.insert(self.prompt_cursor, character);
                    self.prompt_cursor += character.len_utf8();
                }
            }
            GuiAction::Backspace => {
                if let Some(index) = self.prev_prompt_index() {
                    self.prompt_text.drain(index..self.prompt_cursor);
                    self.prompt_cursor = index;
                }
            }
            GuiAction::DeleteChar => {
                if let Some(index) = self.next_prompt_index() {
                    self.prompt_text.drain(self.prompt_cursor..index);
                }
            }
            GuiAction::Left => {
                if let Some(index) = self.prev_prompt_index() {
                    self.prompt_cursor = index;
                }
            }
            GuiAction::Right => {
                if let Some(index) = self.next_prompt_index() {
                    self.prompt_cursor = index;
                }
            }
            GuiAction::TextStart => self.prompt_cursor = 0,
            GuiAction::TextEnd => self.prompt_cursor = self.prompt_text.len(),
            _ => {}
        }
        self.prompt_input = TextState::new(&self.prompt_text);
    }

    fn text_input_kind(&self, module: ModuleId, parameter: usize) -> TextInputKind {
        self.modules()
            .iter()
            .find(|candidate| candidate.id == module)
            .and_then(|module| module.parameter(parameter))
            .map(|parameter| match parameter.value {
                ParameterValue::Time { .. } | ParameterValue::Bars { .. } => TextInputKind::Time,
                ParameterValue::Text(_) => TextInputKind::Free,
                _ => TextInputKind::Number,
            })
            .unwrap_or(TextInputKind::Number)
    }

    fn prev_prompt_index(&self) -> Option<usize> {
        self.prompt_text[..self.prompt_cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
    }

    fn next_prompt_index(&self) -> Option<usize> {
        self.prompt_text[self.prompt_cursor..]
            .char_indices()
            .nth(1)
            .map(|(index, _)| self.prompt_cursor + index)
            .or_else(|| {
                (self.prompt_cursor < self.prompt_text.len()).then_some(self.prompt_text.len())
            })
    }

    fn apply_normal(&mut self, action: GuiAction) {
        match action {
            GuiAction::Quit => {
                if self.dirty {
                    self.mode = Mode::QuitConfirm;
                } else {
                    self.should_quit = true;
                }
            }
            GuiAction::Save => {
                if self.saved_path.is_some() {
                    self.mode = Mode::SaveConfirm;
                } else {
                    self.save_requested = true;
                }
            }
            GuiAction::SaveAs => {
                self.save_requested = true;
            }
            GuiAction::Load => {
                if self.dirty {
                    self.mode = Mode::LoadConfirm;
                } else {
                    self.load_requested = true;
                }
            }
            GuiAction::Export => {
                let text = self
                    .saved_path
                    .as_deref()
                    .map(|path| path.trim_end_matches(".bw").to_string() + ".wav")
                    .unwrap_or_else(|| "output.wav".to_string());
                self.open_prompt(Mode::ExportPrompt, text);
            }
            GuiAction::OpenModules => {
                self.open_modules_requested = true;
            }
            GuiAction::TrackSettings => {
                self.mode = Mode::TrackSettings { parameter: 0 };
            }
            GuiAction::TrackEdit => {
                let text = self.track_text().to_string();
                self.open_prompt(Mode::TrackPrompt, text);
            }
            GuiAction::EditComposition => self.toggle_composition(),
            GuiAction::ExitComposition => self.exit_composition(),
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::OpenPalette => {
                self.mode = Mode::Palette;
                self.palette_searching = false;
                self.palette_filter.clear();
                self.palette_filter_index = 0;
                self.palette_index = 0;
            }
            GuiAction::Palette(category) => self.open_category(category),
            GuiAction::TogglePlay => self.playing = !self.playing,
            GuiAction::ToggleMeters => {
                self.show_meters = !self.show_meters;
                self.sync_audio_patch();
            }
            GuiAction::Undo => {
                if let Some(snapshot) = self.undo.pop() {
                    self.redo.push(self.snapshot());
                    self.restore(snapshot);
                }
            }
            GuiAction::Redo => {
                if let Some(snapshot) = self.redo.pop() {
                    self.undo.push(self.snapshot());
                    self.restore(snapshot);
                }
            }
            GuiAction::Instrument(index) => {
                if index < self.instruments.len() {
                    self.active_instrument = index;
                    self.mode = Mode::Normal;
                    self.sync_audio_patch();
                }
            }
            GuiAction::Delete => {
                if let Some(module) = self.module_at(self.cursor()) {
                    let before = self.snapshot();
                    let id = module.id;
                    self.instrument_mut()
                        .surface_mut()
                        .modules
                        .retain(|module| module.id != id);
                    sync_delay_sources(self.instrument_mut().surface_mut());
                    self.commit(before);
                }
            }
            GuiAction::Edit => {
                if let Some(module) = self.module_at(self.cursor())
                    && module.parameter_count() != 0
                {
                    self.mode = Mode::Edit {
                        module: module.id,
                        parameter: 0,
                    };
                }
            }
            GuiAction::Move => {
                if let Some(module) = self.module_at(self.cursor()) {
                    self.mode = Mode::Move {
                        module: module.id,
                        origin: self.cursor(),
                    };
                }
            }
            GuiAction::Copy => {
                if let Some(module) = self.module_at(self.cursor()) {
                    self.mode = Mode::Copy { source: module.id };
                }
            }
            GuiAction::Select => {
                self.mode = Mode::Select {
                    anchor: self.cursor(),
                };
            }
            GuiAction::Rotate => {
                let cursor = self.cursor();
                if let Some(module) = self.module_at(cursor) {
                    let orientation = match module.orientation {
                        Orientation::Right => Orientation::Down,
                        Orientation::Down => Orientation::Right,
                    };
                    if self.area_fits(
                        module.kind().width(orientation),
                        module.kind().height(orientation),
                        module.position,
                        &[module.id],
                    ) {
                        let id = module.id;
                        let before = self.snapshot();
                        if let Some(module) = self
                            .instrument_mut()
                            .surface_mut()
                            .modules
                            .iter_mut()
                            .find(|module| module.id == id)
                        {
                            module.orientation = orientation;
                            self.commit(before);
                        }
                    }
                }
            }
            GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Confirm
            | GuiAction::Cancel
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::Search
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_quit_confirm(&mut self, action: GuiAction) {
        match action {
            GuiAction::Confirm | GuiAction::Copy => self.should_quit = true,
            GuiAction::Cancel | GuiAction::OpenPalette => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn toggle_composition(&mut self) {
        let target = self
            .module_at(self.cursor())
            .filter(|module| module.is_composition())
            .map(|module| module.id);
        if let Some(target) = target {
            self.enter_composition_surface(target);
            self.mode = Mode::Normal;
        } else {
            self.exit_composition_surface();
            self.mode = Mode::Normal;
        }
    }

    fn enter_composition_surface(&mut self, owner: ModuleId) {
        let cursor = self.cursor();
        let inst = self.instrument_mut();
        inst.composition_stack
            .push((inst.editing_composition, cursor));
        inst.editing_composition = Some(owner);
        inst.surface_mut().cursor = GridPos::new(0, 0);
    }

    fn exit_composition_surface(&mut self) {
        let inst = self.instrument_mut();
        if inst.editing_composition.is_none() {
            return;
        }
        if let Some((parent, cursor)) = inst.composition_stack.pop() {
            inst.editing_composition = parent;
            inst.surface_mut().cursor = cursor;
        } else {
            inst.editing_composition = None;
        }
    }

    fn exit_composition(&mut self) {
        self.exit_composition_surface();
        self.mode = Mode::Normal;
    }

    fn apply_palette(&mut self, action: GuiAction) {
        if self.palette_searching {
            self.apply_palette_search(action);
            return;
        }

        match action {
            GuiAction::PaletteLeft | GuiAction::Left => self.move_palette_category(-1),
            GuiAction::PaletteRight | GuiAction::Right => self.move_palette_category(1),
            GuiAction::Palette(category) => self.open_category(category),
            GuiAction::PaletteUp | GuiAction::Up => self.move_palette_selection(-1),
            GuiAction::PaletteDown | GuiAction::Down => self.move_palette_selection(1),
            GuiAction::Confirm | GuiAction::OpenPalette => {
                let module = self.selected_palette_choice();
                self.insert_palette_module(module);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::Search => {
                self.palette_filter.clear();
                self.palette_filter_index = 0;
                self.palette_searching = true;
            }
            GuiAction::LeftFast
            | GuiAction::DownFast
            | GuiAction::UpFast
            | GuiAction::RightFast
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Quit
            | GuiAction::OpenModules
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::EditComposition
            | GuiAction::ExitComposition
            | GuiAction::Delete
            | GuiAction::Move
            | GuiAction::Copy
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_palette_search(&mut self, action: GuiAction) {
        match action {
            GuiAction::Cancel => {
                self.palette_searching = false;
                self.palette_filter.clear();
                self.palette_filter_index = 0;
            }
            GuiAction::Backspace => {
                self.palette_filter.pop();
                self.palette_filter_index = 0;
            }
            GuiAction::Down => {
                if self.palette_filter_index + 1 < self.filtered_palette_modules().len() {
                    self.palette_filter_index += 1;
                }
            }
            GuiAction::Up => {
                self.palette_filter_index = self.palette_filter_index.saturating_sub(1);
            }
            GuiAction::Confirm => {
                if let Some(module) = self.selected_filtered_palette_choice() {
                    self.insert_palette_module(module);
                }
                self.palette_searching = false;
                self.palette_filter.clear();
                self.palette_filter_index = 0;
                self.mode = Mode::Normal;
            }
            GuiAction::InputChar(character) => {
                self.palette_filter.push(character);
                self.palette_filter_index = 0;
            }
            _ => {}
        }
    }

    fn apply_move(&mut self, action: GuiAction, module: ModuleId, origin: GridPos) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Confirm | GuiAction::Move | GuiAction::Copy => {
                let cursor = self.cursor();
                if let Some(mut held) = self.held_move.take() {
                    let before = held.before.clone();
                    held.module.position = self
                        .bounded_module_position(&held.module, cursor.x as i16, cursor.y as i16)
                        .unwrap_or(cursor);
                    self.instrument_mut()
                        .surface_mut()
                        .modules
                        .push(held.module);
                    self.commit(before);
                    self.mode = Mode::Normal;
                    return;
                }
                let Some(found) = self.modules().iter().find(|found| found.id == module) else {
                    self.mode = Mode::Normal;
                    return;
                };
                let next = self.moved_position(found, found.position, origin, cursor);
                let before = self.snapshot();
                if let Some(found) = self
                    .instrument_mut()
                    .surface_mut()
                    .modules
                    .iter_mut()
                    .find(|found| found.id == module)
                {
                    found.position = next;
                    self.commit(before);
                }
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel => {
                if let Some(mut held) = self.held_move.take() {
                    let origin = held.origin;
                    held.module.position = origin;
                    let inst = self.instrument_mut();
                    inst.editing_composition = held.origin_surface;
                    inst.composition_stack = held.origin_stack;
                    inst.surface_mut().cursor = origin;
                    inst.surface_mut().modules.push(held.module);
                    self.mode = Mode::Normal;
                    return;
                }
                self.instrument_mut().surface_mut().cursor = origin;
                self.mode = Mode::Normal;
            }
            GuiAction::EditComposition => self.move_across_composition(module, origin),
            GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::OpenModules
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::ExitComposition
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::OpenPalette
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn move_across_composition(&mut self, module: ModuleId, origin: GridPos) {
        let target = self
            .module_at(self.cursor())
            .filter(|candidate| candidate.is_composition() && candidate.id != module)
            .map(|candidate| candidate.id);
        let can_exit = self.instrument().editing_composition.is_some();
        if target.is_none() && !can_exit {
            return;
        }

        if self.held_move.is_none() {
            let before = self.snapshot();
            let origin_surface = self.instrument().editing_composition;
            let origin_stack = self.instrument().composition_stack.clone();
            let Some(index) = self
                .instrument()
                .surface()
                .modules
                .iter()
                .position(|candidate| candidate.id == module)
            else {
                self.mode = Mode::Normal;
                return;
            };
            let module_value = self.instrument_mut().surface_mut().modules.remove(index);
            self.held_move = Some(HeldMove {
                module: module_value,
                origin_surface,
                origin_stack,
                origin,
                before,
            });
        }

        if let Some(target) = target {
            self.enter_composition_surface(target);
        } else {
            self.exit_composition_surface();
        }
        self.mode = Mode::Move { module, origin };
    }

    fn apply_copy(&mut self, action: GuiAction, source: ModuleId) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Confirm | GuiAction::Move | GuiAction::Copy => {
                let cursor = self.cursor();
                let Some(module) = self.modules().iter().find(|module| module.id == source) else {
                    self.mode = Mode::Normal;
                    return;
                };
                let before = self.snapshot();
                let position = self
                    .bounded_module_position(module, cursor.x as i16, cursor.y as i16)
                    .unwrap_or(cursor);
                let mut next_module_id = self.next_module_id;
                let copy = module.duplicate(position, &mut next_module_id);
                self.instrument_mut().surface_mut().modules.push(copy);
                sync_delay_sources(self.instrument_mut().surface_mut());
                self.next_module_id = next_module_id;
                self.commit(before);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel => self.mode = Mode::Normal,
            GuiAction::OpenPalette
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::OpenModules
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditComposition
            | GuiAction::ExitComposition
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_edit(&mut self, action: GuiAction, module: ModuleId, parameter: usize) {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            self.mode = Mode::Normal;
            return;
        };
        let module_kind = self.modules()[module_index].kind();
        let parameter_count = self.modules()[module_index].parameter_count();
        let special = module_kind.special_editor();
        let total = parameter_count + usize::from(special.is_some());
        if total == 0 {
            self.mode = Mode::Normal;
            return;
        }
        let parameter = parameter.min(total - 1);

        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::Confirm => {
                if parameter == parameter_count {
                    match special {
                        Some(SpecialEditor::Adsr) => {
                            self.mode = Mode::AdsrEdit {
                                module,
                                parameter: 0,
                            };
                        }
                        Some(SpecialEditor::Envelope) => {
                            self.mode = Mode::EnvEdit {
                                module,
                                point: 0,
                                editing: false,
                            };
                        }
                        Some(SpecialEditor::Probe) => self.mode = Mode::ProbeEdit { module },
                        Some(SpecialEditor::Sample) => {
                            self.mode = Mode::SampleView {
                                module,
                                zoom: 1,
                                offset: 0,
                            };
                        }
                        None => {}
                    }
                }
            }
            GuiAction::Down => {
                self.mode = Mode::Edit {
                    module,
                    parameter: (parameter + 1) % total,
                };
            }
            GuiAction::Up => {
                self.mode = Mode::Edit {
                    module,
                    parameter: if parameter == 0 {
                        total - 1
                    } else {
                        parameter - 1
                    },
                };
            }
            GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast => {
                if parameter >= parameter_count {
                    return;
                }
                let before = self.snapshot();
                let up = matches!(action, GuiAction::ValueUp | GuiAction::ValueUpFast);
                let fast = matches!(action, GuiAction::ValueDownFast | GuiAction::ValueUpFast);
                let step_scale = self.step_scale();
                let changed = {
                    let module = &mut self.instrument_mut().surface_mut().modules[module_index];
                    let Some(mut row) = module.parameter(parameter) else {
                        return;
                    };
                    let changed = row.value.adjust(up, fast, step_scale);
                    if changed
                        && matches!(
                            row.value,
                            ParameterValue::Float { .. }
                                | ParameterValue::Time { .. }
                                | ParameterValue::Bars { .. }
                        )
                    {
                        row.connected = false;
                    }
                    if changed {
                        module.set_parameter(parameter, row);
                    }
                    changed
                };
                if changed {
                    self.commit(before);
                }
            }
            GuiAction::TogglePort => {
                if parameter >= parameter_count {
                    return;
                }
                let before = self.snapshot();
                let changed = {
                    let module = &mut self.instrument_mut().surface_mut().modules[module_index];
                    let Some(mut row) = module.parameter(parameter) else {
                        return;
                    };
                    if matches!(
                        row.value,
                        ParameterValue::Float { .. }
                            | ParameterValue::Time { .. }
                            | ParameterValue::Bars { .. }
                    ) {
                        row.connected = !row.connected;
                        module.set_parameter(parameter, row);
                        true
                    } else {
                        false
                    }
                };
                if changed {
                    self.commit(before);
                }
            }
            GuiAction::CycleUnit => {
                if parameter >= parameter_count {
                    return;
                }
                let before = self.snapshot();
                let changed = {
                    let module = &mut self.instrument_mut().surface_mut().modules[module_index];
                    let Some(mut row) = module.parameter(parameter) else {
                        return;
                    };
                    let changed = row.value.cycle_unit();
                    if changed {
                        module.set_parameter(parameter, row);
                    }
                    changed
                };
                if changed {
                    self.commit(before);
                }
            }
            GuiAction::CycleStep => self.step_size = (self.step_size + 1) % 5,
            GuiAction::TypeValue => {
                if parameter < parameter_count
                    && self.modules()[module_index]
                        .parameter(parameter)
                        .map(|parameter| parameter.value.accepts_typed_value())
                        .unwrap_or(false)
                {
                    self.prompt_text = self.modules()[module_index]
                        .parameter(parameter)
                        .map(|parameter| parameter.value.input_text())
                        .unwrap_or_default();
                    self.prompt_cursor = self.prompt_text.len();
                    self.mode = Mode::ValueInput { module, parameter };
                }
            }
            GuiAction::TogglePlay => self.playing = !self.playing,
            GuiAction::Left => self.apply_edit(GuiAction::ValueDown, module, parameter),
            GuiAction::Right => self.apply_edit(GuiAction::ValueUp, module, parameter),
            GuiAction::LeftFast => self.apply_edit(GuiAction::ValueDownFast, module, parameter),
            GuiAction::RightFast => self.apply_edit(GuiAction::ValueUpFast, module, parameter),
            GuiAction::DownFast | GuiAction::UpFast => {}
            GuiAction::OpenPalette
            | GuiAction::ToggleMeters
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Delete
            | GuiAction::Move
            | GuiAction::Copy
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::Quit
            | GuiAction::OpenModules
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditComposition
            | GuiAction::ExitComposition
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_value_input(&mut self, action: GuiAction, module: ModuleId, parameter: usize) {
        match action {
            GuiAction::Cancel => {
                self.mode = Mode::Edit { module, parameter };
            }
            GuiAction::Confirm => {
                let before = self.snapshot();
                let changed = self.set_typed_value(module, parameter);
                if changed {
                    self.commit(before);
                }
                self.mode = Mode::Edit { module, parameter };
            }
            GuiAction::Left
            | GuiAction::Right
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd => {
                self.apply_text_action(action, self.text_input_kind(module, parameter));
            }
            _ => {}
        }
    }

    fn apply_load_confirm(&mut self, action: GuiAction) {
        match action {
            GuiAction::Confirm => {
                if let Some(path) = self.saved_path.clone() {
                    if self.save_project(Path::new(&path)) {
                        self.mode = Mode::Normal;
                        self.load_requested = true;
                    }
                } else {
                    self.save_requested = true;
                    self.load_after_save = true;
                    self.mode = Mode::Normal;
                }
            }
            GuiAction::Delete => {
                self.mode = Mode::Normal;
                self.load_requested = true;
            }
            GuiAction::Cancel => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn apply_save_confirm(&mut self, action: GuiAction) {
        match action {
            GuiAction::Confirm => {
                if let Some(path) = self.saved_path.clone() {
                    self.save_project(Path::new(&path));
                }
                self.mode = Mode::Normal;
            }
            GuiAction::SaveAs => {
                self.save_requested = true;
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn apply_export_prompt(&mut self, action: GuiAction) {
        match action {
            GuiAction::Cancel => self.mode = Mode::Normal,
            GuiAction::Confirm => {
                let path = self.prompt_text.trim();
                if path.is_empty() {
                    self.mode = Mode::Normal;
                    return;
                }
                if self.path_exists(path) {
                    self.mode = Mode::ExportConfirm;
                } else {
                    self.exported_path = Some(path.to_string());
                    self.mode = Mode::Normal;
                }
            }
            GuiAction::Up => {
                self.export_loops = (self.export_loops + 1).min(100);
            }
            GuiAction::Down => {
                self.export_loops = self.export_loops.saturating_sub(1).max(1);
            }
            GuiAction::Left
            | GuiAction::Right
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd => self.apply_text_action(action, TextInputKind::Free),
            _ => {}
        }
    }

    fn apply_export_confirm(&mut self, action: GuiAction) {
        match action {
            GuiAction::Confirm | GuiAction::Copy => {
                let path = self.prompt_text.trim();
                if !path.is_empty() {
                    self.exported_path = Some(path.to_string());
                }
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel | GuiAction::OpenPalette => self.mode = Mode::ExportPrompt,
            _ => {}
        }
    }

    fn apply_track_prompt(&mut self, action: GuiAction) {
        match action {
            GuiAction::Cancel => self.mode = Mode::Normal,
            GuiAction::Confirm => {
                let before = self.snapshot();
                let text = self.prompt_text.trim().to_string();
                self.instrument_mut().track_text = text;
                self.commit(before);
                self.sync_audio_track();
                self.mode = Mode::Normal;
            }
            GuiAction::Left
            | GuiAction::Right
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd => self.apply_text_action(action, TextInputKind::Free),
            GuiAction::Up
            | GuiAction::Down
            | GuiAction::LeftFast
            | GuiAction::DownFast
            | GuiAction::UpFast
            | GuiAction::RightFast
            | GuiAction::OpenPalette
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Quit
            | GuiAction::OpenModules
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditComposition
            | GuiAction::ExitComposition
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Move
            | GuiAction::Copy
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_track_settings(&mut self, action: GuiAction, parameter: usize) {
        let parameter = parameter.min(2);
        match action {
            GuiAction::Cancel | GuiAction::TrackSettings | GuiAction::Confirm => {
                self.mode = Mode::Normal;
            }
            GuiAction::Down => {
                self.mode = Mode::TrackSettings {
                    parameter: (parameter + 1) % 3,
                };
            }
            GuiAction::Up => {
                self.mode = Mode::TrackSettings {
                    parameter: if parameter == 0 { 2 } else { parameter - 1 },
                };
            }
            GuiAction::ValueUp | GuiAction::Right => match parameter {
                0 => self.bpm = (self.bpm + 5).min(300),
                1 => self.scale_index = (self.scale_index + 1) % SCALE_NAMES.len(),
                2 => self.probe_voice = (self.probe_voice + 1) % NUM_VOICES,
                _ => {}
            },
            GuiAction::ValueDown | GuiAction::Left => match parameter {
                0 => self.bpm = self.bpm.saturating_sub(5).max(20),
                1 => {
                    self.scale_index = if self.scale_index == 0 {
                        SCALE_NAMES.len() - 1
                    } else {
                        self.scale_index - 1
                    };
                }
                2 => {
                    self.probe_voice = if self.probe_voice == 0 {
                        NUM_VOICES - 1
                    } else {
                        self.probe_voice - 1
                    };
                }
                _ => {}
            },
            GuiAction::ValueUpFast => {
                if parameter == 0 {
                    self.bpm = (self.bpm + 20).min(300);
                }
            }
            GuiAction::ValueDownFast => {
                if parameter == 0 {
                    self.bpm = self.bpm.saturating_sub(20).max(20);
                }
            }
            GuiAction::TogglePlay => self.playing = !self.playing,
            _ => {}
        }
    }

    fn set_typed_value(&mut self, module: ModuleId, parameter: usize) -> bool {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            return false;
        };
        let Some(mut row) = self.instrument().surface().modules[module_index].parameter(parameter)
        else {
            return false;
        };
        let Some(value) = typed_parameter_value(&row.value, self.prompt_text.trim()) else {
            return false;
        };
        if row.value == value && !row.connected {
            return false;
        }
        row.value = value;
        row.connected = false;
        self.instrument_mut().surface_mut().modules[module_index].set_parameter(parameter, row)
    }

    fn apply_adsr_edit(&mut self, action: GuiAction, module: ModuleId, parameter: usize) {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            self.mode = Mode::Normal;
            return;
        };
        let parameter = parameter.min(1);
        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::Down => {
                self.mode = Mode::AdsrEdit {
                    module,
                    parameter: (parameter + 1) % 2,
                };
            }
            GuiAction::Up => {
                self.mode = Mode::AdsrEdit {
                    module,
                    parameter: if parameter == 0 { 1 } else { 0 },
                };
            }
            GuiAction::ValueUp
            | GuiAction::ValueDown
            | GuiAction::ValueUpFast
            | GuiAction::ValueDownFast => {
                let before = self.snapshot();
                let param_index = parameter + 2;
                let changed = {
                    let module = &mut self.instrument_mut().surface_mut().modules[module_index];
                    let Some(mut row) = module.parameter(param_index) else {
                        return;
                    };
                    let changed = match (&mut row.value, action) {
                        (ParameterValue::Float { value, max, .. }, GuiAction::ValueUp) => {
                            let next = (*value + 5).min(*max);
                            let changed = next != *value;
                            *value = next;
                            changed
                        }
                        (ParameterValue::Float { value, min, .. }, GuiAction::ValueDown) => {
                            let next = (*value - 5).max(*min);
                            let changed = next != *value;
                            *value = next;
                            changed
                        }
                        (ParameterValue::Float { value, max, .. }, GuiAction::ValueUpFast) => {
                            let changed = *value != *max;
                            *value = *max;
                            changed
                        }
                        (ParameterValue::Float { value, min, .. }, GuiAction::ValueDownFast) => {
                            let changed = *value != *min;
                            *value = *min;
                            changed
                        }
                        _ => false,
                    };
                    if changed {
                        module.set_parameter(param_index, row);
                    }
                    changed
                };
                if changed {
                    self.commit(before);
                }
            }
            GuiAction::TogglePlay => self.playing = !self.playing,
            GuiAction::OpenPalette
            | GuiAction::ToggleMeters
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::OpenModules
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditComposition
            | GuiAction::ExitComposition
            | GuiAction::Confirm
            | GuiAction::Delete
            | GuiAction::Move
            | GuiAction::Copy
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::Left
            | GuiAction::Right
            | GuiAction::LeftFast
            | GuiAction::RightFast
            | GuiAction::DownFast
            | GuiAction::UpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_env_edit(&mut self, action: GuiAction, module: ModuleId, point: usize, editing: bool) {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            self.mode = Mode::Normal;
            return;
        };
        let point_count = self.modules()[module_index].env_points().len();
        if point_count == 0 {
            self.mode = Mode::Normal;
            return;
        }
        let point = point.min(point_count - 1);

        if editing {
            match action {
                GuiAction::Cancel | GuiAction::Confirm | GuiAction::Move => {
                    self.mode = Mode::EnvEdit {
                        module,
                        point,
                        editing: false,
                    };
                }
                GuiAction::Left
                | GuiAction::Right
                | GuiAction::LeftFast
                | GuiAction::RightFast
                | GuiAction::Up
                | GuiAction::Down
                | GuiAction::UpFast
                | GuiAction::DownFast => {
                    let before = self.snapshot();
                    let delta = self.step_scale();
                    let moved_point = {
                        let Some(points) = self.instrument_mut().surface_mut().modules
                            [module_index]
                            .env_points_mut()
                        else {
                            self.mode = Mode::Normal;
                            return;
                        };
                        match action {
                            GuiAction::Left => points[point].time -= delta,
                            GuiAction::Right => points[point].time += delta,
                            GuiAction::LeftFast => points[point].time -= delta * 10,
                            GuiAction::RightFast => points[point].time += delta * 10,
                            GuiAction::Up => points[point].value += delta,
                            GuiAction::Down => points[point].value -= delta,
                            GuiAction::UpFast => points[point].value = 100,
                            GuiAction::DownFast => points[point].value = -100,
                            _ => {}
                        }
                        points[point].time = points[point].time.clamp(0, 100);
                        points[point].value = points[point].value.clamp(-100, 100);
                        let target_time = points[point].time;
                        points.sort_by_key(|point| point.time);
                        points
                            .iter()
                            .position(|point| point.time == target_time)
                            .unwrap_or(0)
                    };
                    self.commit(before);
                    self.mode = Mode::EnvEdit {
                        module,
                        point: moved_point,
                        editing: true,
                    };
                }
                GuiAction::CycleStep => self.step_size = (self.step_size + 1) % 5,
                GuiAction::TogglePlay => self.playing = !self.playing,
                _ => {}
            }
            return;
        }

        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::Right => {
                self.mode = Mode::EnvEdit {
                    module,
                    point: (point + 1) % point_count,
                    editing: false,
                };
            }
            GuiAction::Left => {
                self.mode = Mode::EnvEdit {
                    module,
                    point: if point == 0 {
                        point_count - 1
                    } else {
                        point - 1
                    },
                    editing: false,
                };
            }
            GuiAction::Move => {
                self.mode = Mode::EnvEdit {
                    module,
                    point,
                    editing: true,
                };
            }
            GuiAction::ToggleCurve => {
                let before = self.snapshot();
                let Some(points) =
                    self.instrument_mut().surface_mut().modules[module_index].env_points_mut()
                else {
                    self.mode = Mode::Normal;
                    return;
                };
                let curve = &mut points[point].curve;
                *curve = !*curve;
                self.commit(before);
            }
            GuiAction::AddPoint => {
                let before = self.snapshot();
                let new_index = {
                    let Some(points) =
                        self.instrument_mut().surface_mut().modules[module_index].env_points_mut()
                    else {
                        self.mode = Mode::Normal;
                        return;
                    };
                    let current = points[point];
                    let next_time = points.get(point + 1).map(|next| next.time).unwrap_or(100);
                    let time = ((current.time + next_time) / 2).clamp(0, 100);
                    points.push(EnvPoint {
                        time,
                        value: current.value,
                        curve: false,
                    });
                    points.sort_by_key(|point| point.time);
                    points
                        .iter()
                        .position(|point| point.time == time)
                        .unwrap_or(point)
                };
                self.commit(before);
                self.mode = Mode::EnvEdit {
                    module,
                    point: new_index,
                    editing: false,
                };
            }
            GuiAction::DeletePoint => {
                if point_count > 2 {
                    let before = self.snapshot();
                    let Some(points) =
                        self.instrument_mut().surface_mut().modules[module_index].env_points_mut()
                    else {
                        self.mode = Mode::Normal;
                        return;
                    };
                    points.remove(point);
                    self.commit(before);
                    self.mode = Mode::EnvEdit {
                        module,
                        point: point.min(point_count - 2),
                        editing: false,
                    };
                }
            }
            GuiAction::CycleStep => self.step_size = (self.step_size + 1) % 5,
            GuiAction::TogglePlay => self.playing = !self.playing,
            _ => {}
        }
    }

    fn apply_probe_edit(&mut self, action: GuiAction, _module: ModuleId) {
        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::ValueUp | GuiAction::Right | GuiAction::Up => {
                self.probe_len = (self.probe_len * 2).min(44_100 * 2);
            }
            GuiAction::ValueDown | GuiAction::Left | GuiAction::Down => {
                self.probe_len = (self.probe_len / 2).max(8);
            }
            GuiAction::ValueUpFast => {
                self.probe_len = (self.probe_len * 4).min(44_100 * 2);
            }
            GuiAction::ValueDownFast => {
                self.probe_len = (self.probe_len / 4).max(8);
            }
            GuiAction::Delete => self.probe_len = 4410,
            GuiAction::TogglePlay => self.playing = !self.playing,
            _ => {}
        }
    }

    fn apply_sample_view(&mut self, action: GuiAction, module: ModuleId, zoom: u16, offset: u16) {
        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::ValueUp | GuiAction::Up => {
                let zoom = (zoom * 2).min(64);
                let max_offset = 100_u16.saturating_sub(100 / zoom);
                self.mode = Mode::SampleView {
                    module,
                    zoom,
                    offset: offset.min(max_offset),
                };
            }
            GuiAction::ValueDown | GuiAction::Down => {
                let zoom = (zoom / 2).max(1);
                let max_offset = 100_u16.saturating_sub(100 / zoom);
                self.mode = Mode::SampleView {
                    module,
                    zoom,
                    offset: offset.min(max_offset),
                };
            }
            GuiAction::Left => {
                let step = (10 / zoom.max(1)).max(1);
                self.mode = Mode::SampleView {
                    module,
                    zoom,
                    offset: offset.saturating_sub(step),
                };
            }
            GuiAction::Right => {
                let step = (10 / zoom.max(1)).max(1);
                let max_offset = 100_u16.saturating_sub(100 / zoom.max(1));
                self.mode = Mode::SampleView {
                    module,
                    zoom,
                    offset: (offset + step).min(max_offset),
                };
            }
            GuiAction::Delete => {
                self.mode = Mode::SampleView {
                    module,
                    zoom: 1,
                    offset: 0,
                };
            }
            GuiAction::TogglePlay => self.playing = !self.playing,
            _ => {}
        }
    }

    fn apply_select(&mut self, action: GuiAction, anchor: GridPos) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Move | GuiAction::Confirm => {
                self.mode = Mode::SelectMove {
                    anchor,
                    extent: self.cursor(),
                    origin: self.cursor(),
                };
            }
            GuiAction::Copy => {
                if !self.selected_modules().is_empty() {
                    self.mode = Mode::CopySelection {
                        anchor,
                        extent: self.cursor(),
                        origin: self.cursor(),
                    };
                }
            }
            GuiAction::Delete => {
                let ids = self.selected_modules();
                let before = self.snapshot();
                self.instrument_mut()
                    .surface_mut()
                    .modules
                    .retain(|module| !ids.contains(&module.id));
                self.commit(before);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel | GuiAction::Select => self.mode = Mode::Normal,
            GuiAction::OpenPalette
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::OpenModules
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditComposition
            | GuiAction::ExitComposition
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_select_move(
        &mut self,
        action: GuiAction,
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
    ) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Confirm | GuiAction::Move | GuiAction::Copy => {
                let cursor = self.cursor();
                let dx = cursor.x as i16 - origin.x as i16;
                let dy = cursor.y as i16 - origin.y as i16;
                if let Some(mut held) = self.held_selection.take() {
                    let before = held.before.clone();
                    let targets = held
                        .modules
                        .iter()
                        .map(|module| {
                            self.bounded_module_position(
                                module,
                                module.position.x as i16 + dx,
                                module.position.y as i16 + dy,
                            )
                            .unwrap_or(module.position)
                        })
                        .collect::<Vec<_>>();
                    for (module, position) in held.modules.iter_mut().zip(targets) {
                        module.position = position;
                    }
                    self.instrument_mut()
                        .surface_mut()
                        .modules
                        .extend(held.modules);
                    self.commit(before);
                    self.mode = Mode::Normal;
                    return;
                }
                let ids = self.selected_modules();
                let targets: Vec<(ModuleId, GridPos)> = self
                    .modules()
                    .iter()
                    .filter(|module| ids.contains(&module.id))
                    .map(|module| {
                        (
                            module.id,
                            self.bounded_module_position(
                                module,
                                module.position.x as i16 + dx,
                                module.position.y as i16 + dy,
                            )
                            .unwrap_or(module.position),
                        )
                    })
                    .collect();
                let before = self.snapshot();
                for (id, position) in targets {
                    if let Some(module) = self
                        .instrument_mut()
                        .surface_mut()
                        .modules
                        .iter_mut()
                        .find(|module| module.id == id)
                    {
                        module.position = position;
                    }
                }
                self.commit(before);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel => {
                if let Some(held) = self.held_selection.take() {
                    let inst = self.instrument_mut();
                    inst.editing_composition = held.origin_surface;
                    inst.composition_stack = held.origin_stack;
                    inst.surface_mut().cursor = held.origin;
                    inst.surface_mut().modules.extend(held.modules);
                    self.mode = Mode::Normal;
                    return;
                }
                self.instrument_mut().surface_mut().cursor = origin;
                self.mode = Mode::Normal;
            }
            GuiAction::EditComposition => {
                self.move_selection_across_composition(anchor, extent, origin);
            }
            GuiAction::OpenPalette
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::OpenModules
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::ExitComposition
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {
                self.mode = Mode::SelectMove {
                    anchor,
                    extent,
                    origin,
                };
            }
        }
    }

    fn move_selection_across_composition(
        &mut self,
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
    ) {
        let selected = self
            .held_selection
            .as_ref()
            .map(|held| {
                held.modules
                    .iter()
                    .map(|module| module.id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                self.modules()
                    .iter()
                    .filter(|module| {
                        module.position.x >= anchor.x
                            && module.position.x <= extent.x
                            && module.position.y >= anchor.y
                            && module.position.y <= extent.y
                    })
                    .map(|module| module.id)
                    .collect()
            });
        let target = self
            .module_at(self.cursor())
            .filter(|candidate| candidate.is_composition() && !selected.contains(&candidate.id))
            .map(|candidate| candidate.id);
        let can_exit = self.instrument().editing_composition.is_some();
        if target.is_none() && !can_exit {
            return;
        }

        if self.held_selection.is_none() {
            let before = self.snapshot();
            let origin_surface = self.instrument().editing_composition;
            let origin_stack = self.instrument().composition_stack.clone();
            let mut modules = Vec::new();
            let surface = self.instrument_mut().surface_mut();
            let mut index = 0;
            while index < surface.modules.len() {
                if selected.contains(&surface.modules[index].id) {
                    modules.push(surface.modules.remove(index));
                } else {
                    index += 1;
                }
            }
            if modules.is_empty() {
                self.mode = Mode::Normal;
                return;
            }
            self.held_selection = Some(HeldSelection {
                modules,
                origin_surface,
                origin_stack,
                origin,
                before,
            });
        }

        if let Some(target) = target {
            self.enter_composition_surface(target);
        } else {
            self.exit_composition_surface();
        }
        self.mode = Mode::SelectMove {
            anchor,
            extent,
            origin,
        };
    }

    fn apply_copy_selection(
        &mut self,
        action: GuiAction,
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
    ) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Confirm | GuiAction::Move | GuiAction::Copy => {
                let ids = self.selected_modules();
                let cursor = self.cursor();
                let dx = cursor.x as i16 - origin.x as i16;
                let dy = cursor.y as i16 - origin.y as i16;
                let mut next_module_id = self.next_module_id;
                let mut copies = self
                    .modules()
                    .iter()
                    .filter(|module| ids.contains(&module.id))
                    .map(|module| {
                        let x = (module.position.x as i16 + dx)
                            .clamp(0, self.grid_size.width().saturating_sub(1) as i16);
                        let y = (module.position.y as i16 + dy)
                            .clamp(0, self.grid_size.height().saturating_sub(1) as i16);
                        module.duplicate(GridPos::new(x as u16, y as u16), &mut next_module_id)
                    })
                    .collect::<Vec<_>>();
                let copied_ids = self
                    .modules()
                    .iter()
                    .filter(|module| ids.contains(&module.id))
                    .map(|module| module.id)
                    .zip(copies.iter().map(|module| module.id))
                    .collect::<Vec<_>>();
                for copy in &mut copies {
                    let ModuleBody::DelayTap { source, .. } = &mut copy.body else {
                        continue;
                    };
                    source.selected = source.selected.and_then(|selected| {
                        copied_ids
                            .iter()
                            .find_map(|(old, new)| (*old == selected).then_some(*new))
                            .or(Some(selected))
                    });
                }
                let before = self.snapshot();
                for copy in copies {
                    self.instrument_mut().surface_mut().modules.push(copy);
                }
                sync_delay_sources(self.instrument_mut().surface_mut());
                self.next_module_id = next_module_id;
                self.commit(before);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel => self.mode = Mode::Normal,
            GuiAction::OpenPalette
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::OpenModules
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditComposition
            | GuiAction::ExitComposition
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {
                self.mode = Mode::CopySelection {
                    anchor,
                    extent,
                    origin,
                };
            }
        }
    }
}
