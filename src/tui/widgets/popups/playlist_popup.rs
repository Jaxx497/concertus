use crate::{
    tui::widgets::POPUP_PADDING,
    ui_state::{Pane, PlaylistAction, PopupType, UiState},
};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Style, Stylize},
    text::{Line, Text},
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
    let theme = state.get_theme(&Pane::Popup);
    let padding_h = (area.height as f32 * 0.3) as u16;
    let padding_w = (area.width as f32 * 0.2) as u16;

    let block = Block::bordered()
        .title(" Create New Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(theme.border_type)
        .border_style(Style::new().fg(theme.border))
        .fg(theme.text_focused)
        .bg(theme.bg)
        .padding(Padding {
            left: padding_w,
            right: padding_w,
            top: padding_h,
            bottom: 0,
        });

    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::vertical([Constraint::Max(2), Constraint::Length(3)]).split(inner);

    Paragraph::new("Enter playlist title: ")
        .centered()
        .render(chunks[0], buf);

    state.popup.input.set_block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .fg(theme.border)
            .padding(Padding::horizontal(2)),
    );
    state
        .popup
        .input
        .set_style(Style::new().fg(theme.text_focused));
    state.popup.input.render(chunks[1], buf);
}

fn render_add_song_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let theme = state.get_theme(&Pane::Popup);
    let list_items = state
        .playlists
        .iter()
        .map(|p| {
            let playlist_name = p.name.to_string();
            Line::from(playlist_name).fg(theme.text_faded).centered()
        })
        .collect::<Vec<Line>>();

    let block = Block::bordered()
        .title(" Add To Playlist ")
        .title_bottom(" [Enter] / [c]reate playlist / [Esc] ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(theme.border_type)
        .border_style(Style::new().fg(theme.text_secondary))
        .bg(theme.bg)
        .padding(POPUP_PADDING);

    let list = List::new(list_items)
        .block(block)
        .scroll_padding(area.height as usize - 5)
        .highlight_style(Style::new().fg(theme.highlight));

    StatefulWidget::render(list, area, buf, &mut state.popup.selection);
}

fn render_delete_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let theme = state.get_theme(&Pane::Popup);
    let block = Block::bordered()
        .title(format!(" Delete Playlist "))
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(theme.border_type)
        .border_style(Style::new().fg(theme.border))
        .fg(theme.text_focused)
        .bg(theme.bg)
        .padding(Padding {
            left: 5,
            right: 5,
            top: (area.height as f32 * 0.35) as u16,
            bottom: 0,
        });

    if let Some(p) = state.get_selected_playlist() {
        let p_name = Line::from_iter([p.name.as_str().fg(theme.border), " ?".into()]);
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
    let theme = state.get_theme(&Pane::Popup);
    let padding_h = (area.height as f32 * 0.25) as u16;
    let padding_w = (area.width as f32 * 0.2) as u16;

    let block = Block::bordered()
        .title(" Rename Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(Alignment::Center)
        .border_type(theme.border_type)
        .border_style(Style::new().fg(theme.border))
        .fg(theme.text_focused)
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
        let p_name = playlist.name.as_str().fg(theme.border);
        Paragraph::new(Text::from_iter([
            format!("Enter a new name for\n").into(),
            p_name,
        ]))
        .centered()
        .render(chunks[0], buf);

        state.popup.input.set_block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .fg(theme.border)
                .padding(Padding::horizontal(2)),
        );

        state
            .popup
            .input
            .set_style(Style::new().fg(theme.text_focused));
        state.popup.input.render(chunks[1], buf);
    }
}
