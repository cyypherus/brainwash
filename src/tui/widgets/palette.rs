use super::util::{set_cell, set_str};
use crate::tui::module::{ModuleCategory, ModuleKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct PaletteWidget<'a> {
    selected_category: usize,
    selected_module: usize,
    filter: &'a str,
    filtered_modules: Vec<ModuleKind>,
    filter_selection: usize,
    searching: bool,
}

impl<'a> PaletteWidget<'a> {
    pub fn new() -> Self {
        Self {
            selected_category: 0,
            selected_module: 0,
            filter: "",
            filtered_modules: Vec::new(),
            filter_selection: 0,
            searching: false,
        }
    }

    pub fn selected_category(mut self, idx: usize) -> Self {
        self.selected_category = idx;
        self
    }

    pub fn selected_module(mut self, idx: usize) -> Self {
        self.selected_module = idx;
        self
    }

    pub fn filter(
        mut self,
        filter: &'a str,
        modules: Vec<ModuleKind>,
        selection: usize,
        searching: bool,
    ) -> Self {
        self.filter = filter;
        self.filtered_modules = modules;
        self.filter_selection = selection;
        self.searching = searching;
        self
    }
}

impl Widget for PaletteWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let desc_style = Style::default().fg(Color::Gray).bg(Color::Rgb(40, 40, 40));
        let desc_y = area.y + area.height.saturating_sub(1);
        let max_w = area.width as usize;

        if self.searching {
            let mut y = area.y;
            let filter_style = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD);
            set_str(buf, area.x, y, "/", filter_style);
            set_str(
                buf,
                area.x + 1,
                y,
                self.filter,
                Style::default().fg(Color::White),
            );
            y += 2;

            let label_style = Style::default().fg(Color::DarkGray);
            let selected_style = Style::default().fg(Color::Black).bg(Color::Yellow);

            for (i, kind) in self.filtered_modules.iter().enumerate() {
                if y >= area.y + area.height - 1 {
                    break;
                }
                let style = if i == self.filter_selection {
                    selected_style
                } else {
                    label_style
                };
                set_str(buf, area.x + 2, y, kind.name(), style);
                y += 1;
            }

            if let Some(kind) = self.filtered_modules.get(self.filter_selection) {
                let desc: String = kind.description().chars().take(max_w).collect();
                for x in 0..area.width {
                    set_cell(buf, area.x + x, desc_y, ' ', desc_style);
                }
                set_str(buf, area.x, desc_y, &desc, desc_style);
            }
        } else {
            let categories = ModuleCategory::all();
            let mut y = area.y;

            for (cat_idx, cat) in categories.iter().enumerate() {
                if y >= desc_y {
                    break;
                }

                let is_selected_cat = cat_idx == self.selected_category;
                let cat_style = if is_selected_cat {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                set_str(buf, area.x, y, &format!(" {} ", cat.name()), cat_style);
                y += 1;

                if is_selected_cat {
                    let mods = ModuleKind::by_category(*cat);
                    for (mod_idx, kind) in mods.iter().enumerate() {
                        if y >= desc_y {
                            break;
                        }
                        let is_sel = mod_idx == self.selected_module;
                        let style = if is_sel {
                            Style::default().fg(Color::Black).bg(kind.color())
                        } else {
                            Style::default().fg(kind.color())
                        };
                        set_str(buf, area.x + 1, y, &format!(" {} ", kind.name()), style);
                        y += 1;
                    }
                    if let Some(selected_kind) = mods.get(self.selected_module) {
                        let desc: String =
                            selected_kind.description().chars().take(max_w).collect();
                        for x in 0..area.width {
                            set_cell(buf, area.x + x, desc_y, ' ', desc_style);
                        }
                        set_str(buf, area.x, desc_y, &desc, desc_style);
                    }
                }
            }
        }
    }
}
