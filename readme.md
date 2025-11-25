# Concertus - v0.1.0

Concertus is a lightweight, plug and play, TUI music player written in Rust.

![concertus.png](./docs/header.png)

## Usage

To try Concertus, clone the repository and run the program with `cargo run
--release`. 

Begin by assigning one or more 'root' (or 'parent') directories when promted.
The root management window can be managed by pressing the ``` ` ``` key.
Concertus will walk through the supplied folder(s), and create a virtual
library based on the valid files it finds.

It's recommended that users have ffmpeg installed for waveform visualization.
This dependency however is not mandatory.

For the full list of keymaps, refer to the [keymaps
documentation](./docs/keymaps.md). \
For information on custom themeing, refer
to the [themeing specification](./docs/themes.md).

Currently, concertus supports the following filetypes: ```mp3, m4a, flac, ogg, wav```

## Disclaimers

Concertus never writes to user files and does not have any online capabilities,
however, the program relies on accurate tagging. It's strongly recommended that
users ensure their libraries are properly tagged with a tool like
[MP3Tag](https://www.mp3tag.de/en/). 

> **Tip:** Concertus supports live updates by pressing `Ctrl+u` or `F5`

## Known bugs

1. Symphonia/Rodio Related*
    1. There are no reliable Rodio compatible OPUS decoders.
    1. Seeking can be potentially unstable.
    1. Gapless playback is not viable for the time being.

> **Note:** This project is heavily reliant on the Symphonia and Rodio crates.
Many of the playback related issues are due to upstream issues in the
aforementioned libraries. Following several QOL additons, I intend to explore
new backend options. 

## TODO 

- Display more song info in window (user controlled)
- Improved testing for various formats
- Implement a secondary backend (likely mpv)

## Other

Concertus is a hobby project primary written for educational purposes. This
project seeks to demonstrate my understanding of a series of programming
fundamentals, including but not limited to multi-threading, atomics, string
interning, database integration, de/serialization, memory management, integrity
hashing, session persistence, OS operations, modular design, view models, 
state management, user customization, and much more. 
