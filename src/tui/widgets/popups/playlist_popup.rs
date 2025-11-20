use crate::{
    tui::widgets::POPUP_PADDING,
    ui_state::{Pane, PlaylistAction, PopupType, UiState, fade_color},
};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, List, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};

pub struct PlaylistPopup;
impl StatefulWidget for PlaylistPopup {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if let PopupType::Playlist(action) = &state.popup.current {
            match action {
                PlaylistAction::Create | PlaylistAction::CreateWithSongs => {
                    render_create_popup(area, buf, state)
                }
                PlaylistAction::AddSong => render_add_song_popup(area, buf, state),
                PlaylistAction::Delete => render_delete_popup(area, buf, state),
                PlaylistAction::Rename => render_rename_popup(area, buf, state),
            }
        }
    }
}

fn render_create_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let focus = matches!(state.get_pane(), Pane::Popup);
    let theme = state.theme_manager.get_display_theme(focus);
    let padding_h = (area.height as f32 * 0.3) as u16;
    let padding_w = (area.width as f32 * 0.2) as u16;

    let block = Block::bordered()
        .border_type(theme.border_type)
        .border_style(theme.border)
        .title(" Create New Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .padding(Padding {
            left: padding_w,
            right: padding_w,
            top: padding_h,
            bottom: 0,
        })
        .fg(theme.accent)
        .bg(theme.bg);

    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::vertical([Constraint::Max(2), Constraint::Length(3)]).split(inner);

    Paragraph::new("Enter playlist title: ")
        .centered()
        .render(chunks[0], buf);

    state.popup.input.set_block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .padding(Padding::horizontal(2)),
    );
    state
        .popup
        .input
        .set_style(Style::new().fg(theme.text_primary));
    state.popup.input.render(chunks[1], buf);
}

fn render_add_song_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let focus = matches!(state.get_pane(), Pane::Popup);
    let theme = state.theme_manager.get_display_theme(focus);
    let list_items = state
        .playlists
        .iter()
        .map(|p| {
            let playlist_name = p.name.to_string();
            Line::from(playlist_name)
                .fg(fade_color(theme.dark, theme.text_muted, 0.85))
                .centered()
        })
        .collect::<Vec<Line>>();

    let block = Block::bordered()
        .border_type(theme.border_type)
        .border_style(theme.border)
        .title(" Add To Playlist ")
        .title_bottom(" [Enter] / [c]reate playlist / [Esc] ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .padding(POPUP_PADDING)
        .bg(theme.bg);

    if list_items.is_empty() {
        state.popup.selection.select(None);
        return Paragraph::new("\nThere are no playlists!\n\nCreate a playlist by pressing [c]")
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(block.clone())
            .fg(theme.accent)
            .render(area, buf);
    }

    let list = List::new(list_items)
        .block(block)
        .scroll_padding(area.height as usize - 5)
        .highlight_style(theme.selection);

    StatefulWidget::render(list, area, buf, &mut state.popup.selection);
}

fn render_delete_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let focus = matches!(state.get_pane(), Pane::Popup);
    let theme = state.theme_manager.get_display_theme(focus);
    let block = Block::bordered()
        .border_type(theme.border_type)
        .border_style(theme.border)
        .title(format!(" Delete Playlist "))
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .padding(Padding {
            left: 5,
            right: 5,
            top: (area.height as f32 * 0.35) as u16,
            bottom: 0,
        })
        .fg(theme.text_primary)
        .bg(theme.bg);

    if let Some(p) = state.get_selected_playlist() {
        let p_name = Line::from_iter([p.name.as_str(), " ?".into()]);
        let warning = Paragraph::new(Text::from_iter([
            format!("Are you sure you want to delete\n").into(),
            p_name,
        ]))
        .block(block)
        .wrap(Wrap { trim: true })
        .centered();
        warning.render(area, buf);
    };
}

fn render_rename_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let focus = matches!(state.get_pane(), Pane::Popup);
    let theme = state.theme_manager.get_display_theme(focus);
    let padding_h = (area.height as f32 * 0.25) as u16;
    let padding_w = (area.width as f32 * 0.2) as u16;

    let block = Block::bordered()
        .title(" Rename Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(Alignment::Center)
        .border_type(theme.border_type)
        .border_style(theme.border)
        .fg(theme.text_primary)
        .bg(theme.bg)
        .padding(Padding {
            left: padding_w,
            right: padding_w,
            top: padding_h,
            bottom: 0,
        });

    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::vertical([Constraint::Max(3), Constraint::Length(3)]).split(inner);

    if let Some(playlist) = state.get_selected_playlist() {
        let p_name = Span::from(playlist.name.as_str());
        Paragraph::new(Text::from_iter([
            format!("Enter a new name for\n").into(),
            p_name,
        ]))
        .centered()
        .render(chunks[0], buf);

        state.popup.input.set_block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(2)),
        );

        state
            .popup
            .input
            .set_style(Style::new().fg(theme.text_primary));
        state.popup.input.render(chunks[1], buf);
    }
}
