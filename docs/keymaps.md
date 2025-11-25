Because Concertus is a modal program, keymaps depend on the specific context in
which they are used. Contexts are defined by a combination of the mode (e.g.
Playlist, Queue, Album, Search) and the Pane (e.g. Main pane, sidebar, popup,
etc.). Global keymaps and playback keymaps will work in almost every context,
with the exception of searching as not to affect a user's search query. 

**Keymaps are case sensitive.**

## Global Keymaps

##### Navigation
| Action      | Keymap |
| ----------- | ----------- |
| Select / Confirm | `Enter`|
| Scroll Up 1 Item     | `k` `↑` |
| Scroll Down 1 Item     | `j` `↓` |
| Scroll Down (5 / 25 Items) | `d` `D`|
| Scroll Up (5 / 25 Items) | `u` `U`|
| Go to Top / Bottom | `g` `G` |

##### Views
| Action      | Keymap |
| ----------- | ----------- |
| Album View |  `1` \| `Ctrl` + `a`|
| Playlist View|  `2` \| `Ctrl` + `t`|
| Queue View | `3` \| `Ctrl` + `q`|
| Change Sidebar Size | `[` `]` |
| Smooth Waveform | `{` `}` |
| Fullscreen Progress View | `f` |
| Oscilloscope View | `o` `O` |
| Waveform View | `w` `W` |
| ProgressBar View | `b` `B` |

##### General
| Action      | Keymap |
| ----------- | ----------- |
| Search | `\`
| Open Settings | ``` ` ``` |
| Clear Popup / Exit Search | `Esc` |
| Update Library | `F5` \| `Ctrl` + `u` |
| Hot Reload Current Theme | `F6` |
| Open Theme Manager | `C`|
| Cycle Themes | `<` `>`|
| Quit | `Ctrl` + `c`|

 > **Note:** The update logic is currently handled in the main thread meaning
 > the UI will hang until the update is complete. This will be addressed in
 > future versions.

## Playback Keymaps
These keymaps will work in most contexts.

| Action      | Keymap |
| ----------- | ----------- |
| Toggle Pause | `Space` |
| Seek Forward (5s / 30s)| `n` `N` |
| Seek Back (5s / 30s)| `p` `P` |
| Play Next in Queue | `Ctrl` + `n`|
| Play Prev in History | `Ctrl` + `p`|
| Stop & Clear Queue | `Ctrl` + `s`|

> **Tip:** To toggle pause while searching or in a popup, use `Ctrl` + `Space`

## Main Pane Keymaps
The main pane is defined as the larger pane on the right where individual songs
are displayed. 

| Action      | Keymap |
| ----------- | ----------- |
| Play Song | `Enter` |
| Queue Song | `q` |
| Add to Playlist | `a` |
| Go to Album | `Ctrl` + `a` |
| Go back to Sidebar | `h` `←`|
> **Add to Playlist Shortcut:** Press `aa` on a song (or selection) to add it to the
> most recently modified playlist, bypassing the popup. 

##### Multi-Selection

| Action      | Keymap |
| ----------- | ----------- |
| Toggle Multi-Selection | `v` |
| Toggle Multi-Selection on all Relevant Items | `V` |
| Clear Multi-Selection | `Ctrl` + `v` |

> **Multi-selection** enables users to select multiple songs to add, queue, or
> remove from a playlist or the queue. Selection order is preserved.

##### Playlist/Queue Specific

| Action      | Keymap |
| ----------- | ----------- |
| Remove Song | `x` |
| Shift Song/Selection Down | `J` |
| Shift Song/Selection Position Up | `K` |
| Shuffle Queue (Queue Mode Only) | `s` |


## Sidebar (Album) Keymaps
These keymaps apply when the album/playlist sidebar is focused. 

| Action      | Keymap |
| ----------- | ----------- |
| Queue Full Entity | `q` |
| Switch to Main Pane | `l` `→` <br> `Enter` |


##### Playlist-View Specific

| Action      | Keymap |
| ----------- | ----------- |
| Create New Playlist | `c` |
| Rename Playlist | `r` |
| Delete Playlist | `D` |


##### Album-View Specific

| Action      | Keymap |
| ----------- | ----------- |
| Toggle Album Sorting Key<br> `Artist` `Album Title` `Year` | `Ctrl` + `h` <br> `Ctrl` + `l` |

> **Note:** Add an entire album or playlist to the queue by pressing `q`
> directly from the sidebar pane. If nothing is playing, then the first element
> of the selected entity will begin playing automatically.
