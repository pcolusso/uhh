use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::app::{App, SafetyStatus};

impl Widget for &App {
    /// Renders the user interface widgets.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Ratio(1, 4), // Top pane (1 part of 4)
                Constraint::Ratio(2, 4), // Middle pane (2 parts of 4)
                Constraint::Ratio(1, 4), // Bottom pane (1 part of 4)
                Constraint::Length(1),   // Status line (fixed 1 line)
            ])
            .split(area);

        // Top pane - editable input
        let top_block = Block::bordered()
            .title("Input")
            .border_type(BorderType::Rounded)
            .style(if self.focused_pane == 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        let top_text = if self.focused_pane == 0 {
            // Show cursor when focused
            let mut display_text = self.input_text.clone();
            display_text.insert(self.input_cursor, '|');
            display_text
        } else {
            self.input_text.clone()
        };

        let top_paragraph = Paragraph::new(top_text.as_str()).block(top_block);

        top_paragraph.render(main_layout[0], buf);

        // Middle pane - twice the size
        let middle_block = Block::bordered()
            .title("Output")
            .border_type(BorderType::Rounded)
            .style(if self.is_loading_completion {
                Style::default().fg(Color::Cyan)
            } else if self.focused_pane == 1 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        let middle_text = if self.focused_pane == 1 {
            // Show cursor when focused
            let mut display_text = self.response_text.clone();
            display_text.insert(self.response_cursor, '|');
            display_text
        } else {
            self.response_text.clone()
        };

        let middle_paragraph = Paragraph::new(middle_text.as_str()).block(middle_block);

        middle_paragraph.render(main_layout[1], buf);

        // Bottom pane (not focusable)
        let bottom_block = Block::bordered()
            .title("Safety Check")
            .border_type(BorderType::Rounded)
            .style(if self.is_loading_safety_check {
                Style::default().fg(Color::Cyan)
            } else {
                match self.safety_status {
                    SafetyStatus::Safe => Style::default().fg(Color::Green),
                    SafetyStatus::Unsafe => Style::default().fg(Color::Red),
                    SafetyStatus::Unknown => Style::default(),
                }
            });

        let bottom_paragraph = Paragraph::new(self.safety_check_text.as_str()).block(bottom_block);

        bottom_paragraph.render(main_layout[2], buf);

        // Status line with dynamic content
        let status_text = if self.is_loading_completion {
            "Loading completion... | Press Up/Down to navigate, Esc to quit"
        } else if self.is_loading_safety_check {
            "Running safety check... | Press Up/Down to navigate, Esc to quit"
        } else {
            match self.safety_status {
                SafetyStatus::Safe => {
                    "Command appears safe | Press Up/Down to navigate, Esc to quit"
                }
                SafetyStatus::Unsafe => {
                    "⚠️ Command may be unsafe | Press Up/Down to navigate, Esc to quit"
                }
                SafetyStatus::Unknown => "Ready | Press Up/Down to navigate, Esc to quit",
            }
        };

        let status_color = if self.is_loading_completion || self.is_loading_safety_check {
            Color::Yellow
        } else {
            match self.safety_status {
                SafetyStatus::Safe => Color::Green,
                SafetyStatus::Unsafe => Color::Red,
                SafetyStatus::Unknown => Color::Green,
            }
        };

        let status_paragraph =
            Paragraph::new(status_text).style(Style::default().bg(status_color).fg(Color::Black));

        status_paragraph.render(main_layout[3], buf);
    }
}
