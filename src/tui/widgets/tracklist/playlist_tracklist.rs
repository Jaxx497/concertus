use super::{get_widths, COLUMN_SPACING, PADDING};
use crate::{
    domain::SongInfo,
    get_readable_duration,
    tui::widgets::tracklist::get_keymaps,
    ui_state::{Pane, UiState},
    DurationStyle,
};
use ratatui::{
    layout::{Alignment, Flex},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{StatefulWidget, *},
};

pub struct PlaylistView;
impl StatefulWidget for PlaylistView {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::TrackList);
        if state.playlists.is_empty() {
            Block::bordered()
                .title_top(Line::from("").alignment(Alignment::Center))
                .borders(theme.border_display)
                .border_type(BorderType::Thick)
                .border_style(Style::default().fg(theme.border))
                .bg(theme.bg)
                .padding(PADDING)
                .render(area, buf);
        }

        let songs = state.legal_songs.as_slice();
        let song_len = songs.len();

        let results = format!(" Playlist Size: {} Songs ", song_len);

        let rows = songs
            .iter()
            .enumerate()
            .map(|(idx, song)| {
                let index = Cell::from(format!("{:>3}", idx + 1)).fg(theme.text_faded);
                let title_col = Cell::from(song.get_title()).fg(theme.text_focused);
                let artist_col = Cell::from(song.get_artist()).fg(theme.text_focused);
                let format_col = Cell::from(song.format.to_string()).fg(theme.text_secondary);
                let duration_str = get_readable_duration(song.duration, DurationStyle::Clean);

                let dur_col =
                    Cell::from(Text::from(duration_str).right_aligned()).fg(theme.text_focused);

                Row::new([index, title_col, artist_col, format_col, dur_col])
            })
            .collect::<Vec<Row>>();

        let widths = get_widths(&state.get_mode());

        let block = Block::bordered()
            .title_top(Line::from(results))
            .title_bottom(get_keymaps(state.get_pane()).fg(theme.text_faded))
            .title_alignment(Alignment::Center)
            .borders(theme.border_display)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(theme.border))
            .bg(theme.bg)
            .padding(PADDING);

        let table = Table::new(rows, widths)
            .column_spacing(COLUMN_SPACING)
            .flex(Flex::Start)
            .block(block)
            // .highlight_symbol(SELECTOR)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(theme.text_highlighted);

        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
