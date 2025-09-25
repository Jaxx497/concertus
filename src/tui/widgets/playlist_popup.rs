use crate::{
    tui::widgets::POPUP_PADDING,
    ui_state::{DARK_GRAY, GOLD, GOOD_RED, PlaylistAction, PopupType, UiState},
};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
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
    let block = Block::bordered()
        .title(" Create New Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
        .bg(Color::Rgb(25, 25, 25))
        .padding(POPUP_PADDING);

    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::vertical([
        Constraint::Max(2),
        Constraint::Max(2),
        Constraint::Length(3),
    ])
    .split(inner);

    Paragraph::new("Enter a name for your new playlist:")
        .centered()
        .render(chunks[1], buf);

    state.popup.input.set_block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .fg(Color::Rgb(220, 220, 100))
            .padding(Padding::horizontal(1)),
    );
    state.popup.input.set_style(Style::new().fg(Color::White));
    state.popup.input.render(chunks[2], buf);
}

fn render_add_song_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let list_items = state
        .playlists
        .iter()
        .map(|p| {
            let playlist_name = p.name.to_string();
            Line::from(playlist_name)
                .fg(Color::Rgb(150, 150, 150))
                .centered()
        })
        .collect::<Vec<Line>>();

    let block = Block::bordered()
        .title(" Select Playlist ")
        .title_bottom(" [Enter] / [c]reate playlist / [Esc] ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(GOOD_RED))
        .bg(DARK_GRAY)
        .padding(POPUP_PADDING);

    let list = List::new(list_items)
        .block(block)
        .scroll_padding(area.height as usize - 5)
        .highlight_style(Style::new().fg(GOLD));

    StatefulWidget::render(list, area, buf, &mut state.popup.selection);
}

fn render_delete_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let block = Block::bordered()
        .title(format!(" Delete Playlist?? "))
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
        .bg(Color::Rgb(25, 25, 25))
        .padding(POPUP_PADDING);

    if let Some(p) = state.get_selected_playlist() {
        let warning = Paragraph::new(format!("Are you sure you want to delete\n[{}]?", p.name))
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
    let block = Block::bordered()
        .title(" Rename Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
        .bg(Color::Rgb(25, 25, 25))
        .padding(POPUP_PADDING);

    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::vertical([
        Constraint::Percentage(10),
        Constraint::Max(3),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .split(inner);

    if let Some(playlist) = state.get_selected_playlist() {
        Paragraph::new(format!("Enter a new name for\n `[{}]`: ", playlist.name))
            .centered()
            .render(chunks[1], buf);

        state.popup.input.set_block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .fg(Color::Rgb(220, 220, 100))
                .padding(Padding::horizontal(1)),
        );

        state.popup.input.set_style(Style::new().fg(Color::White));
        state.popup.input.render(chunks[2], buf);
    }
}
