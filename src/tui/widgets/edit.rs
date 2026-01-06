use super::util::set_str;
use crate::tui::module::{Module, ModuleKind, ModuleParams, ParamKind, StandardModule};
use crate::tui::patch::Patch;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct EditWidget<'a> {
    module: &'a Module,
    selected_param: usize,
    patch: &'a Patch,
    step_label: &'a str,
}

impl<'a> EditWidget<'a> {
    pub fn new(module: &'a Module, selected_param: usize, patch: &'a Patch) -> Self {
        Self {
            module,
            selected_param,
            patch,
            step_label: "1x",
        }
    }

    pub fn step_label(mut self, label: &'a str) -> Self {
        self.step_label = label;
        self
    }
}

impl Widget for EditWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let defs = self.module.kind.param_defs();
        let color = self.module.kind.color();

        let title_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);
        let selected_style = Style::default().fg(Color::Black).bg(color);

        let title = self.module.kind.name();
        set_str(buf, area.x, area.y, title, title_style);

        let desc = self.module.kind.description();
        let desc_style = Style::default().fg(Color::DarkGray);
        set_str(buf, area.x, area.y + 1, desc, desc_style);

        let mut y = area.y + 3;

        for (i, def) in defs.iter().enumerate() {
            if y >= area.y + area.height {
                break;
            }
            let is_selected = i == self.selected_param;
            let is_connected = self.module.params.is_connected(i);

            let style = if is_selected {
                selected_style
            } else {
                label_style
            };

            let port_str = match def.kind {
                ParamKind::Input => "● ",
                ParamKind::Float { .. } | ParamKind::Time => {
                    if is_connected {
                        "● "
                    } else {
                        "✕ "
                    }
                }
                ParamKind::Enum | ParamKind::Toggle | ParamKind::Int { .. } => "  ",
            };
            set_str(buf, area.x, y, port_str, style);
            set_str(buf, area.x + 2, y, def.name, style);

            let val_x = area.x + 8;
            let v_style = if is_selected {
                selected_style
            } else {
                value_style
            };

            match &def.kind {
                ParamKind::Input => {
                    set_str(buf, val_x, y, "(input)", label_style);
                }
                ParamKind::Enum => {
                    let val_str = if let ModuleKind::Standard(StandardModule::DelayTap(delay_id)) =
                        self.module.kind
                    {
                        if i == 0 {
                            let is_valid_delay = self
                                .patch
                                .module(delay_id)
                                .map(|m| m.kind == ModuleKind::Standard(StandardModule::Delay))
                                .unwrap_or(false);
                            if is_valid_delay {
                                if let Some(pos) = self.patch.module_position(delay_id) {
                                    format!("Delay @{},{}", pos.x, pos.y)
                                } else {
                                    "Delay (?)".to_string()
                                }
                            } else {
                                "Invalid".to_string()
                            }
                        } else {
                            self.module
                                .params
                                .enum_display(i)
                                .unwrap_or("?")
                                .to_string()
                        }
                    } else if self.module.kind == ModuleKind::Standard(StandardModule::Sample)
                        && i == 0
                    {
                        if let ModuleParams::Sample {
                            file_name, samples, ..
                        } = &self.module.params
                        {
                            if samples.is_empty() {
                                "(no file)".to_string()
                            } else {
                                file_name.clone()
                            }
                        } else {
                            "?".to_string()
                        }
                    } else {
                        self.module
                            .params
                            .enum_display(i)
                            .unwrap_or("?")
                            .to_string()
                    };
                    set_str(buf, val_x, y, &val_str, v_style);
                    if is_selected {
                        set_str(
                            buf,
                            val_x + val_str.len() as u16 + 1,
                            y,
                            "<hl>",
                            label_style,
                        );
                    }
                }
                ParamKind::Toggle => {
                    let val = self.module.params.get_toggle(i);
                    let val_str = if val { "on" } else { "off" };
                    set_str(buf, val_x, y, val_str, v_style);
                    if is_selected {
                        set_str(
                            buf,
                            val_x + val_str.len() as u16 + 1,
                            y,
                            "<hl>",
                            label_style,
                        );
                    }
                }
                ParamKind::Float { .. } => {
                    let val = self.module.params.get_float(i).unwrap_or(0.0);
                    let val_str = format!("{:.3}", val);
                    set_str(buf, val_x, y, &val_str, v_style);
                    if is_selected {
                        let hint = format!("{} <hl> ;", self.step_label);
                        set_str(buf, val_x + val_str.len() as u16 + 1, y, &hint, label_style);
                    }
                }
                ParamKind::Time => {
                    let val_str = self
                        .module
                        .params
                        .get_time(i)
                        .map(|t| t.display())
                        .unwrap_or_else(|| "?".to_string());
                    set_str(buf, val_x, y, &val_str, v_style);
                    if is_selected {
                        set_str(
                            buf,
                            val_x + val_str.len() as u16 + 1,
                            y,
                            "<hl> u;",
                            label_style,
                        );
                    }
                }
                ParamKind::Int { .. } => {
                    let val = self.module.params.get_int(i).unwrap_or(0);
                    let val_str = format!("{}", val);
                    set_str(buf, val_x, y, &val_str, v_style);
                    if is_selected {
                        set_str(
                            buf,
                            val_x + val_str.len() as u16 + 1,
                            y,
                            "<hl>",
                            label_style,
                        );
                    }
                }
            }

            y += 1;
        }

        if let Some(def) = defs.get(self.selected_param)
            && let Some(desc) = def.desc
        {
            y += 1;
            if y < area.y + area.height {
                set_str(buf, area.x, y, desc, label_style);
            }
        }

        if self.module.kind == ModuleKind::Standard(StandardModule::Sample)
            && let ModuleParams::Sample { samples, .. } = &self.module.params
            && !samples.is_empty()
        {
            let sample_count = samples.len();
            let duration = sample_count as f32 / 44100.0;
            y += 1;
            set_str(
                buf,
                area.x + 2,
                y,
                &format!("{} samp ({:.2}s)", sample_count, duration),
                label_style,
            );
        }

        if let Some(editor_name) = self.module.kind.special_editor_name() {
            y += 1;
            let special_idx = defs.len();
            let is_selected = self.selected_param == special_idx;
            let style = if is_selected {
                selected_style
            } else {
                label_style
            };
            set_str(buf, area.x, y, "▶ ", style);
            set_str(buf, area.x + 2, y, editor_name, style);
            if is_selected {
                set_str(
                    buf,
                    area.x + 2 + editor_name.len() as u16 + 1,
                    y,
                    "<enter>",
                    label_style,
                );
            }
        }
    }
}
