# Concertus - v0.0.3

Concertus is a TUI music player written in Rust. 

## Usage

Simplicity is a wonderful thing. Concertus was inspired by many other
TUI Projects, the unix philosophy, and the brilliance of n/vim. 

Begin by assigning one or more "root" directories of your files.
Concertus will walk through these directories, building a
representation of your library.

The settings and root management window can be opened with the
backtick [ ` ] and tilde [ ~ ] characters. 

## Disclaimers

Concertus never writes to user files, only a database stored in the
config or appdata directory. 

Concertus does not have any online capability. It relies entirely on
the tags of a users library. It's recommended to use tools like MP3Tag
or similar to make the proper modifications. 

## Known bugs

1. Symphonia Related*
    1. m4a files rarely fail to play, but often disc numbers will not
       be displayed.
    2. FLAC will rarely cause crashes when seeking in a song,
       functionality is enabled. 
        OGG files crash on seek without fail, functionality is
        disabled.
    3. Symphonia does not have OPUS capabilities.

2. Live updates can cause panics on occasion. It's almost always due
    to the indexing on the sidebar. In most cases, you should be fine,
    but not all cases are covered. 

3. Accessing deleted songs - Accessing a deleted song through the
   history will likely render the rest of the history playlist
    inaccessible

*Symphonia is a major dependency of this project. Most of the
playback related issues are due to upstream issues in the symphonia
library. I will begin to look for alternative backends. Perhaps ffmpeg
or gstreamer. Metadata may be read by other readers if necessary.


## TODO 

- Fix history bug
- Implement a playlist system (***)
- Search by album/artist (!)
    Should be simple, just need to do it
- Ditch power mode (?)
- Add more settings
    - Update on start?
- Implement a secondary player backend (gstreamer?)

