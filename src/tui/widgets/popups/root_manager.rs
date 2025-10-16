use crate::{
    strip_win_prefix,
    tui::widgets::{POPUP_PADDING, SELECTOR},
    ui_state::{Pane, SettingsMode, UiState},
};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Style, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, HighlightSpacing, List, Padding, Paragraph, StatefulWidget, Widget, Wrap,
    },
};

pub struct RootManager;
impl StatefulWidget for RootManager {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let settings_mode = state.get_settings_mode();

        let theme = state.get_theme(&Pane::Popup);

        let title = match settings_mode {
            Some(SettingsMode::ViewRoots) => " Settings - Music Library Roots ",
            Some(SettingsMode::AddRoot) => " Add New Root Directory ",
            Some(SettingsMode::RemoveRoot) => " Remove Root Directory ",
            None => return,
        };

        let block = Block::bordered()
            .title(title)
            .title_bottom(get_help_text(settings_mode))
            .title_alignment(ratatui::layout::Alignment::Center)
            .border_type(BorderType::Double)
            .border_style(Style::new().fg(theme.border))
            .bg(theme.bg_panel)
            .padding(POPUP_PADDING);

        let inner = block.inner(area);
        block.render(area, buf);

        match settings_mode {
            Some(SettingsMode::ViewRoots) => render_roots_list(inner, buf, state),
            Some(SettingsMode::AddRoot) => render_add_root(inner, buf, state),
            Some(SettingsMode::RemoveRoot) => render_remove_root(inner, buf, state),
            None => (),
        }
    }
}

fn get_help_text(mode: Option<&SettingsMode>) -> &'static str {
    if let Some(m) = mode {
        match m {
            SettingsMode::ViewRoots => " [a]dd / [d]elete / [Esc] close ",
            SettingsMode::AddRoot => " [Enter] confirm / [Esc] cancel ",
            SettingsMode::RemoveRoot => " [Enter] confirm / [Esc] cancel ",
        }
    } else {
        unreachable!()
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
            .wrap(Wrap { trim: true })
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

    let list = List::new(items)
        .highlight_symbol(SELECTOR)
        .highlight_style(state.theme_manager.active.text_highlighted)
        .highlight_spacing(HighlightSpacing::Always);

    ratatui::prelude::StatefulWidget::render(list, area, buf, &mut state.popup.selection);
}

fn render_add_root(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let chunks = Layout::vertical([
        Constraint::Max(3),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .split(area);

    Paragraph::new("Enter the path to a directory containing music files:")
        .wrap(Wrap { trim: false })
        .render(chunks[0], buf);

    let theme = state.get_theme(state.get_pane());

    state.popup.input.set_block(
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
        .popup
        .input
        .set_style(Style::new().fg(theme.text_focused));

    state.popup.input.render(chunks[1], buf);

    let example = Paragraph::new("Example: C:\\Music or /home/user/music")
        .fg(theme.bg_panel)
        .centered();
    example.render(chunks[2], buf);
}

fn render_remove_root(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &UiState,
) {
    let theme = state.get_theme(&Pane::Popup);
    let roots = state.get_roots();

    if roots.is_empty() {
        Paragraph::new("No root selected")
            .centered()
            .render(area, buf);
        return;
    }
    let selected_root = &roots[state.popup.selection.selected().unwrap()];
    let selected_root = strip_win_prefix(&selected_root);

    let warning = Paragraph::new(format!(
        "Are you sure you want to remove this root?\n\n{}\n\nThis will remove all songs from this directory from your library.",
        selected_root
    ))
    .wrap(Wrap { trim: true })
    .centered()
    .fg(theme.text_secondary);

    warning.render(area, buf);
}
