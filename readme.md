# Concertus - v0.0.5a

Concertus is a lightweight, simple to use TUI music player written in Rust.

![concertus.png](https://i.postimg.cc/GmfgdZt7/concertus-img.png)

## Usage

Begin by assigning one or more 'root' (or 'parent') directories when
promted. The root management window can be managed by pressing the ```
` ``` key. Concertus will walk through the supplied folder(s), and
create a library based on the valid files it finds.

It's recommended that users have ffmpeg and a nerd font installed for
visual flare. Neither is mandatory, however.

Concertus does not leverage any online capabilities, and strictly
relies on the accurate and proper tagging. It's strongly recommended
to utilize a tool like [MP3Tag](https://www.mp3tag.de/en/) to ensure
the entries of one's library are accurate. Libraries can be live
refreshed simply by pressing ```F5``` or ```Ctrl-u```

Currently, supported filetypes include the following: ```mp3, m4a, flac, wav, ogg```

## Disclaimers

Concertus never writes to user files, only a SQLite database stored in
the users local config or appdata directory. 

## Known bugs

1. Symphonia/Rodio Related*
    1. FLAC may cause crashes when seeking in a song. 
    1. OGG files cannot utilize seek functionality. 
    1. There are no reliable Rodio compatible OPUS decoders.

2. Waveforms may generate on songs that cannot be played.

*This project is heavily reliant on the Symphonia and Rodio crates.
Most of the playback related issues are due to upstream issues in the
aforementioned libraries. Following the rollout of the playlist
feature, alongside other small QOL additions, new backend options will
be explored. However, lots of progress is being made in the rodio
library, which may solve this problem in time. 

## Current Development Objectives
- Implementing a playlist system
 - Bulk Selection (Implemented- need to add it for sidebar view)
- Change order of songs in playlists and queue
 - Create new playlist on add song to playlist popup

## TODO 
- Tune search results
- Re-work sort-by-column approach
- Provide visual progress when scanning in songs
- Add more settings
    - Custom themeing
    - Update on start?
- Implement a secondary player backend (gstreamer?)

## Other
This project seeks to demonstrate my understanding of a series of
programming fundamentals, including but not limited to multi-threaded
management, atomics, string internment, database integration,
de/serialization, integrity hashing, session persistence, modular
design, view models, and state management. 
