# Concertus - v0.0.3

Concertus is a TUI music player written in Rust. 

## Usage

Simplicity is a wonderful thing. Concertus was inspired by many other
TUI Projects, the unix philosophy, and the brilliance of n/vim. 

Begin by assigning one or more "root" directories of your files.
Concertus will walk through these directories, building a
representation of your library.


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
    2. FLAC can cause crashes when seeking in a song. OPUS files crash
       every time, so the functionality is blocked.
    3. Symphonia/Rodio do not have OPUS decoders.

2. Live updates can cause panics on occasion. In most cases, you
   should be fine, but not all cases are covered. 

*Symphonia is a major dependency of this project. Most of the
playback related issues are due to upstream issues in the symphonia
library. I will begin to look for alternative backends. Perhaps ffmpeg
or gstreamer. Metadata may be read by other readers if necessary.


## TODO 

- Settings window (!!!)
- Implement a playlist system (***)
- Search by album/artist (!)
- Ditch power mode (?)

