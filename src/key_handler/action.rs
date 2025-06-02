use crate::{
    ui_state::{Mode, Pane, SettingsMode, UiState},
    REFRESH_RATE,
};
use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::{collections::HashSet, sync::LazyLock, time::Duration};

static ILLEGAL_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| HashSet::from(['\'', ';']));
const C: KeyModifiers = KeyModifiers::CONTROL;

const SEEK_SMALL: usize = 5;
const SEEK_LARGE: usize = 30;
const SCROLL_MID: usize = 5;
const SCROLL_XTRA: usize = 50;

#[derive(PartialEq, Eq)]
pub enum Action {
    // Player Controls
    Play,
    Stop,
    TogglePause,
    PlayNext,
    PlayPrev,
    SeekForward(usize),
    SeekBack(usize),

    // Queue Controls
    QueueSong,
    QueueAlbum,
    RemoveFromQueue,

    // Updating App State
    UpdateLibrary,
    SendSearch,
    UpdateSearch(KeyEvent),
    SortColumnsNext,
    SortColumnsPrev,
    ToggleAlbumSort(bool),
    ChangeMode(Mode),
    ChangePane(Pane),
    GoToAlbum,
    Scroll(Director),

    // Errors, Convenience & Other
    ViewSettings,
    SettingsUp,
    SettingsDown,
    RootAdd,
    RootRemove,
    RootConfirm,
    // SettingsCancel,
    SettingsInput(KeyEvent),

    HandleErrors,
    SoftReset,
    QUIT,
}

#[derive(PartialEq, Eq)]
pub enum Director {
    Up(usize),
    Down(usize),
    Top,
    Bottom,
}

use KeyCode::*;

#[rustfmt::skip]
pub fn handle_key_event(key_event: KeyEvent, state: &UiState) -> Option<Action> {
    
    let pane = state.get_pane();

    if let Some(action) = global_commands(&key_event, pane) {
        return Some(action)
    } 

    match pane {
        Pane::TrackList => handle_main_pane(&key_event),
        Pane::Search    => handle_search_pane(&key_event),
        Pane::SideBar   => handle_sidebar_pane(&key_event),
        Pane::Popup     => handle_popup_pane(&key_event, state),
    }
}

fn global_commands(key: &KeyEvent, pane: &Pane) -> Option<Action> {
    let in_search = pane == Pane::Search;
    let in_settings = pane == Pane::Popup;

    // Works on every pane, even search
    match (key.modifiers, key.code) {
        (_, Esc) => Some(Action::SoftReset),
        (C, Char('c')) => Some(Action::QUIT),
        (_, Char('`')) | (_, Char('~')) => Some(Action::ViewSettings),
        (C, Char('u')) | (_, F(5)) => Some(Action::UpdateLibrary),
        (C, Char(' ')) => Some(Action::TogglePause),

        // Works on everything except search
        _ if (!in_search && !in_settings) => match (key.modifiers, key.code) {
            // PLAYBACK COMMANDS
            (_, Char(' ')) => Some(Action::TogglePause),
            (C, Char('s')) => Some(Action::Stop),
            (C, Char('n')) => Some(Action::PlayNext),
            (C, Char('p')) => Some(Action::PlayPrev),
            (_, Char('n')) => Some(Action::SeekForward(SEEK_SMALL)),
            (_, Char('N')) => Some(Action::SeekForward(SEEK_LARGE)),
            (_, Char('p')) => Some(Action::SeekBack(SEEK_SMALL)),
            (_, Char('P')) => Some(Action::SeekBack(SEEK_LARGE)),

            // NAVIGATION
            (_, Char('/')) => Some(Action::ChangeMode(Mode::Search)),
            (C, Char('q')) => Some(Action::ChangeMode(Mode::Queue)),

            // SCROLLING
            (_, Char('j')) | (_, Down) => Some(Action::Scroll(Director::Down(1))),
            (_, Char('k')) | (_, Up) => Some(Action::Scroll(Director::Up(1))),
            (_, Char('d')) => Some(Action::Scroll(Director::Down(SCROLL_MID))),
            (_, Char('u')) => Some(Action::Scroll(Director::Up(SCROLL_MID))),
            (_, Char('D')) => Some(Action::Scroll(Director::Down(SCROLL_XTRA))),
            (_, Char('U')) => Some(Action::Scroll(Director::Up(SCROLL_XTRA))),
            (_, Char('g')) => Some(Action::Scroll(Director::Top)),
            (_, Char('G')) => Some(Action::Scroll(Director::Bottom)),

            _ => None,
        },
        _ => None,
    }
}

fn handle_main_pane(key: &KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        // QUEUEING SONGS
        (_, Char('q')) => Some(Action::QueueSong),
        (_, Char('x')) => Some(Action::RemoveFromQueue),

        (C, Char('a')) => Some(Action::GoToAlbum),

        // SORTING SONGS
        (_, Left) | (_, Char('h')) => Some(Action::SortColumnsPrev),
        (_, Right) | (_, Char('l')) => Some(Action::SortColumnsNext),

        (_, Enter) => Some(Action::Play),
        (_, Tab) => Some(Action::ChangeMode(Mode::Album)),
        _ => None,
    }
}

fn handle_sidebar_pane(key: &KeyEvent) -> Option<Action> {
    match key.code {
        Char('q') => Some(Action::QueueAlbum),
        Enter | Tab => Some(Action::ChangePane(Pane::TrackList)),
        Left | Char('h') => Some(Action::ToggleAlbumSort(false)),
        Right | Char('l') => Some(Action::ToggleAlbumSort(true)),

        _ => None,
    }
}

fn handle_search_pane(key: &KeyEvent) -> Option<Action> {
    match key.code {
        Tab | Char('/') | Enter => Some(Action::SendSearch),

        Char(x) if ILLEGAL_CHARS.contains(&x) => None,
        _ => Some(Action::UpdateSearch(*key)),
    }
}

fn handle_popup_pane(key: &KeyEvent, state: &UiState) -> Option<Action> {
    if let Some(_) = state.get_error() {
        match key.code {
            Char('?') | Char('`') | Enter | Esc => Some(Action::SoftReset),
            _ => None,
        };
    }

    match state.get_settings_mode() {
        SettingsMode::ViewRoots => match key.code {
            Char('a') => Some(Action::RootAdd),
            Char('r') => Some(Action::RootRemove),
            Up | Char('k') => Some(Action::SettingsUp),
            Down | Char('j') => Some(Action::SettingsDown),
            _ => None,
        },
        SettingsMode::AddRoot => match key.code {
            Esc => Some(Action::ViewSettings),
            Enter => Some(Action::RootConfirm),
            _ => Some(Action::SettingsInput(*key)),
        },
        SettingsMode::RemoveRoot => match key.code {
            Esc => Some(Action::ViewSettings),
            Enter => Some(Action::RootConfirm),
            _ => None,
        },
    }
}

pub fn next_event() -> Result<Option<Event>> {
    match event::poll(Duration::from_millis(REFRESH_RATE))? {
        true => Ok(Some(event::read()?)),
        false => Ok(None),
    }
}
