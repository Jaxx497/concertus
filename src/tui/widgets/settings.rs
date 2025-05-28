use crate::{
    strip_win_prefix,
    ui_state::{Pane, SettingsMode, UiState, GOOD_RED},
};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, HighlightSpacing, List, Padding, Paragraph, StatefulWidget, Widget, Wrap,
    },
};

use super::SELECTOR;

static POPUP_PADDING: Padding = Padding {
    left: 2,
    right: 2,
    top: 2,
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
            .title(title)
            .title_bottom(get_help_text(&settings_mode))
            .title_alignment(ratatui::layout::Alignment::Center)
            .border_type(BorderType::Double)
            .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
            .bg(Color::Rgb(25, 25, 25))
            .padding(POPUP_PADDING);

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
    state: &mut UiState,
) {
    let roots = state.get_roots();

    if roots.is_empty() {
        Paragraph::new("No music library configured.\nPress 'a' to add a parent directory.")
            .centered()
            .render(area, buf);
        return;
    }

    let items: Vec<Line> = roots
        .iter()
        .map(|r| {
            let root = strip_win_prefix(r);
            Line::from(root)
        })
        .collect();

    let theme = state.get_theme(&Pane::Popup);

    let list = List::new(items)
        .highlight_style(Style::default().fg(Color::Black).bg(theme.text_highlighted))
        .highlight_symbol(SELECTOR)
        .highlight_spacing(HighlightSpacing::Always);

    ratatui::prelude::StatefulWidget::render(list, area, buf, &mut state.settings_selection);
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
            .fg(theme.text_highlighted)
            .padding(Padding {
                left: 1,
                right: 1,
                top: 0,
                bottom: 0,
            }),
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

    if roots.is_empty() {
        Paragraph::new("No root selected")
            .centered()
            .render(area, buf);
        return;
    }
    let selected_root = &roots[state.settings_selection.selected().unwrap()];
    let selected_root = strip_win_prefix(&selected_root);

    let warning = Paragraph::new(format!(
        "Are you sure you want to remove this root?\n\n{}\n\nThis will remove all songs from this directory from your library.",
        selected_root
    ))
    .wrap(Wrap { trim: true })
    .centered()
    .fg(GOOD_RED);

    warning.render(area, buf);
}
