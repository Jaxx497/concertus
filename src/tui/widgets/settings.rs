use crate::{
    strip_win_prefix,
    ui_state::{Pane, SettingsMode, UiState},
};
use ratatui::{
    layout::{Constraint, Layout, Margin},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, Padding, Paragraph, StatefulWidget, Widget},
};

static PADDING: Padding = Padding {
    left: 2,
    right: 2,
    top: 1,
    bottom: 1,
};

pub struct Settings;

impl StatefulWidget for Settings {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let settings_mode = state.get_settings_mode();

        let title = match settings_mode {
            SettingsMode::ViewRoots => " Settings - Music Library Roots ",
            SettingsMode::AddRoot => " Add New Root Directory ",
            SettingsMode::RemoveRoot => " Remove Root Directory ",
        };

        let block = Block::bordered()
            .border_type(BorderType::Double)
            .title(title)
            .title_bottom(get_help_text(&settings_mode))
            .title_alignment(ratatui::layout::Alignment::Center)
            .padding(PADDING);

        let inner = block.inner(area);
        block.render(area, buf);

        match settings_mode {
            SettingsMode::ViewRoots => render_roots_list(inner, buf, state),
            SettingsMode::AddRoot => render_add_root(inner, buf, state),
            SettingsMode::RemoveRoot => render_remove_root(inner, buf, state),
        }
    }
}

fn get_help_text(mode: &SettingsMode) -> &'static str {
    match mode {
        SettingsMode::ViewRoots => " [a]dd / [r]emove / [Esc] close ",
        SettingsMode::AddRoot => " [Enter] confirm / [Esc] cancel ",
        SettingsMode::RemoveRoot => " [Enter] confirm / [Esc] cancel ",
    }
}

fn render_roots_list(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &UiState,
) {
    let roots = state.get_roots();

    if roots.is_empty() {
        Paragraph::new("No music library roots configured.\nPress 'a' to add a directory.")
            .centered()
            .render(area, buf);
        return;
    }

    let items: Vec<ListItem> = roots
        .iter()
        .enumerate()
        .map(|(idx, root)| {
            let root = strip_win_prefix(root);

            let content = if idx == state.settings_selection {
                Line::from(vec![
                    Span::from("→ ").fg(Color::Yellow),
                    Span::from(root).fg(Color::White),
                ])
            } else {
                Line::from(vec![Span::from("  "), Span::from(root).fg(Color::Gray)])
            };
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::BOLD));

    ratatui::prelude::Widget::render(list, area, buf);
}

fn render_add_root(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .split(area);

    Paragraph::new("Enter the path to a directory containing music files:").render(chunks[0], buf);

    let theme = state.get_theme(&Pane::Popup);

    state.new_root_input.set_block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .fg(theme.text_highlighted),
    );
    state
        .new_root_input
        .set_style(Style::new().fg(theme.text_focused));

    state.new_root_input.render(chunks[1], buf);

    let example = Paragraph::new("Example: C:\\Music or /home/user/music")
        .fg(Color::DarkGray)
        .centered();
    example.render(chunks[2], buf);
}

fn render_remove_root(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &UiState,
) {
    let roots = state.get_roots();

    if roots.is_empty() || state.settings_selection >= roots.len() {
        Paragraph::new("No root selected")
            .centered()
            .render(area, buf);
        return;
    }

    let selected_root = &roots[state.settings_selection];

    let warning = Paragraph::new(format!(
        "Are you sure you want to remove this root?\n\n{}\n\nThis will remove all songs from this directory from your library.",
        selected_root
    ))
    .wrap(ratatui::widgets::Wrap { trim: true })
    .centered()
    .fg(Color::Red);

    warning.render(area, buf);
}
