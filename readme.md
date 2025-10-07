# Concertus - v0.0.8a

Concertus is a lightweight, simple to use TUI music player written in Rust.

![concertus.png](./docs/header.png)

## Usage

Begin by assigning one or more 'root' (or 'parent') directories when promted.
The root management window can be managed by pressing the ``` ` ``` key.
Concertus will walk through the supplied folder(s), and create a library based
on the valid files it finds.

It's recommended that users have ffmpeg and a nerd font installed for visual
flare. Neither is mandatory, however.

>For a full list of keymaps, please [view the keymaps
documentation](./docs/keymaps.md).

Currently, supported filetypes include the following: ```mp3, m4a, flac, wav,
ogg```

## Disclaimers

Concertus never writes to user files and does not have any online capabilities,
but relies on accurate tagging. It's strongly recommended that users ensure
their libraries are properly tagged with a tool like
[MP3Tag](https://www.mp3tag.de/en/). Update a library during runtime with
`Ctrl+u`

## Known bugs

1. Symphonia/Rodio Related*
    1. There are no reliable Rodio compatible OPUS decoders.
    1. Seeking can be potentially unstable.
    1. Gapless playback is not viable.
    1. Waveforms may generate on songs that cannot be played.

> **Note:** This project is heavily reliant on the Symphonia and Rodio crates.
Many of the playback related issues are due to upstream issues in the
aforementioned libraries. Following several QOL additons, I intend to explore
new backend options. However, lots of progress is being made within the rodio
crate, which may solve several of these problems in time. 

## TODO 

- Provide visual progress when scanning in songs
- Custom themeing
- Improved testing for various formats
- Re-work sort-by-column approach
- Implement a secondary player backend (gstreamer?)

## Other

This project seeks to demonstrate my understanding of a series of programming
fundamentals, including but not limited to multi-threaded management, atomics,
string internment, database integration, de/serialization, memory management,
integrity hashing, session persistence, OS operations, modular design, view
models, and state management. 
