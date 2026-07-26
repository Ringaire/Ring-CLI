use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};
use uuid::Uuid;

use crate::tui::theme::{UI, MUTED, ACCENT};

use super::core::scroll_list::{anchor_above, label, pointer, ScrollList};

const MAX_VISIBLE: usize = 10;

pub struct AgentEntry {
    pub id:      Uuid,
    pub label:   String,
    pub model:   String,
    pub running: bool,
}

pub struct AgentPickerModal {
    entries: Vec<AgentEntry>,
    list:    ScrollList,
}

impl AgentPickerModal {
    pub fn new(entries: Vec<AgentEntry>) -> Self {
        let mut list = ScrollList::new(MAX_VISIBLE);
        if !entries.is_empty() {
            list.focus(0);
        }
        Self { entries, list }
    }

    pub fn move_up(&mut self) {
        self.list.up(self.entries.len());
    }

    pub fn move_down(&mut self) {
        self.list.down(self.entries.len());
    }

    pub fn selected_id(&self) -> Option<Uuid> {
        self.entries.get(self.list.cursor()).map(|e| e.id)
    }

    pub fn height(&self) -> u16 {
        if self.entries.is_empty() {
            3
        } else {
            self.list.body_height(self.entries.len(), 1)
        }
    }

    pub fn area(parent: Rect, y: u16, h: u16) -> Rect {
        anchor_above(parent, y, h)
    }

    pub fn render(&self) -> Paragraph<'static> {
        let dim = Style::default().fg(MUTED);

        if self.entries.is_empty() {
            return Paragraph::new(vec![
                Line::from(Span::styled("Agents", Style::default().fg(UI).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("No sub-agents active", dim)),
            ]);
        }

        let mut lines: Vec<Line<'static>> = Vec::new();
        let n = self.entries.len();
        lines.push(Line::from(vec![
            Span::styled("Agents  ", Style::default().fg(UI).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} active  •  ↑↓ navigate  Enter view  Esc back", n), dim),
        ]));

        lines.extend(self.list.render_rows(&self.entries, dim, |e, rs| {
            let status_icon = if e.running { "◉" } else { "○" };
            let status_color = if e.running { ACCENT } else { MUTED };
            let text = format!("{} {} · {}", status_icon, e.label, e.model);
            Line::from(vec![
                pointer(rs.selected),
                Span::styled(status_icon, Style::default().fg(status_color)),
                Span::raw(" "),
                label(rs.selected, text),
            ])
        }));

        Paragraph::new(lines)
    }

    pub fn clear() -> Clear {
        Clear
    }
}
