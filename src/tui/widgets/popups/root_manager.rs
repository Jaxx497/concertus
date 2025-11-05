use crate::{
    strip_win_prefix,
    tui::widgets::SELECTOR,
    ui_state::{SettingsMode, UiState},
};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Style, Stylize},
    text::{Line, Text},
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
        let theme = state.theme_manager.get_display_theme(true);

        let padding_h = (area.height as f32 * 0.2) as u16;
        let padding_w = (area.width as f32 * 0.2) as u16;

        let title = match settings_mode {
            Some(SettingsMode::ViewRoots) => " Settings - Music Library Roots ",
            Some(SettingsMode::AddRoot) => " Add New Root Directory ",
            Some(SettingsMode::RemoveRoot) => " Remove Root Directory ",
            None => return,
        };

        let block = Block::bordered()
            .border_type(theme.border_type)
            .border_style(theme.border)
            .title(title)
            .title_bottom(get_keymaps(settings_mode))
            .title_alignment(ratatui::layout::Alignment::Center)
            .padding(Padding {
                left: padding_w,
                right: padding_w,
                top: padding_h,
                bottom: 0,
            })
            .bg(theme.bg);

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

fn get_keymaps(mode: Option<&SettingsMode>) -> &'static str {
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
    let theme = state.theme_manager.get_display_theme(true);

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
        .fg(state.theme_manager.active.text_primary)
        .highlight_symbol(SELECTOR)
        .highlight_style(Style::new().fg(theme.text_selected).bg(theme.selection))
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
        .fg(state.theme_manager.active.accent)
        .wrap(Wrap { trim: true })
        .render(chunks[0], buf);

    let theme = state.theme_manager.get_display_theme(true);

    state.popup.input.set_block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .fg(theme.accent)
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
        .set_style(Style::new().fg(theme.text_primary));

    state.popup.input.render(chunks[1], buf);

    let example = Paragraph::new("Ex: C:\\Music or ~/music/albums")
        .fg(theme.text_muted)
        .centered();
    example.render(chunks[2], buf);
}

fn render_remove_root(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &UiState,
) {
    let theme = state.theme_manager.get_display_theme(true);
    let roots = state.get_roots();

    if roots.is_empty() {
        Paragraph::new("No root selected")
            .centered()
            .render(area, buf);
        return;
    }
    let selected_root = &roots[state.popup.selection.selected().unwrap()];
    let selected_root = strip_win_prefix(&selected_root);

    let text = Text::from_iter([
        Line::from("Are you sure you want to delete:"),
        Line::default(),
        selected_root.fg(theme.accent).into(),
        Line::default(),
        "This will remove all songs from this directory from your library."
            .fg(theme.text_muted)
            .into(),
    ]);

    let warning = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .centered()
        .fg(theme.text_secondary);

    warning.render(area, buf);
}
