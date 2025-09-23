Because Concertus is a modal program, keymaps depend on the specific context in
which they are used. Contexts are defined by a combination of the mode (e.g.
Playlist, Queue, Album, Search) and the Pane (e.g. Main pane, sidebar, popup,
etc.). Global keymaps and playback keymaps will work in almost every context,
with the exception of searching as not to affect a user's search query. 

**Keymaps are case sensitive.**

# Global Keymaps

| Action      | Keymap |
| ----------- | ----------- |
**Navigation**
| Select / Confirm | `Enter`|
| Scroll Up 1 Item     | `k` `↑` |
| Scroll Down 1 Item     | `j` `↓` |
| Scroll Down (5 / 25 Items) | `d` `D`|
| Scroll Up (5 / 25 Items) | `u` `U`|
| Go to Top / Bottom | `g` `G` |
| Smooth Waveform | `[` `]` |
**Views**
| Album View |  `1` \| `Ctrl` + `a`|
| Playlist View|  `2` \| `Ctrl` + `t`|
| View Queue | `3` \| `Ctrl` + `q`|
**General**
| Search | `/`|
| Open Settings | ``` ` ``` |
| Update Library | `F5` \| `Ctrl` + `u` |
| Clear Popup / Search | `Esc`|
| Quit | `Ctrl` + `c`|

 > **Note:** The update logic is currently handled in the main thread meaning the
 UI will hang until the update is complete. This will be addressed in
 future versions.

# Playback Keymaps
These keymaps will work in most contexts.

| Action      | Keymap |
| ----------- | ----------- |
| Toggle Pause | `Space` |
| Seek Forward (5s / 30s)| `n`  `N` |
| Seek Back (5s / 30s)| `p` `P` |
| Play Next in Queue | `Ctrl` + `n`|
| Play Prev in History | `Ctrl` + `p`|
| Stop | `Ctrl` + `s`|

> **Tip:** To toggle pause while searching or in a popup, use `Ctrl` +
> `Space`

## Main Pane Keymaps
The main pane is defined as the larger pane on the right where
individual songs are displayed. 

| Action      | Keymap |
| ----------- | ----------- |
| Play Song | `Enter` |
| Queue Song | `q` |
| Add to Playlist | `a` |
| Go back to Sidebar | `h` `←`|
**Multi-Selection**
| Toggle Multi-Selection | `v` |
| Toggle Multi-Selection on all Relevant Items | `V` |
| Clear Multi-Selection | `Ctrl` + `v` |
**Playlist/Queue Specific**
| Remove Song | `x` |
| Shift Song Position Down | `J` |
| Shift Song Position Up | `K` |

> **Multi-selection** enables users to select multiple songs to queue
> or add to a playlist. Selection order is preserved.

> **Playlist Shortcut:** Press `aa` on a song (or selection) to add it
> to the most recently modified playlist, bypassing the popup. 


## Sidebar (Album) Keymaps

These keymaps apply when the sidebar on the left is focused. 

| Action      | Keymap |
| ----------- | ----------- |
| Queue Full Entity | `q` |
| Switch to Main Pane | `l` `→` <br> `Enter` |
**Playlist-View Specific**
| Create New Playlist | `c` |
| Rename Playlist | `r` |
| Delete Playlist | `Ctrl` + `d` |
**Album-View Specific**
| Toggle Album Sorting Algorithm | `Ctrl` + `h` <br> `Ctrl` + `l` |

> **Note:** Add an entire album or playlist to the queue by pressing
> `q` directly from the sidebar pane. If nothing is playing, the
> selected entity will begin playing automatically.
