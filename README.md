# Marconio

A desktop internet radio that looks and feels like a piece of hardware you'd find in a late-2000s Honda. Marconio streams [NTS Radio](https://www.nts.live) — both live channels and their catalog of mixtapes — through a native audio engine with a handful of audio effects you can toggle on the fly.

The whole thing fits in a small fixed-size window. There's an amber LCD, six preset buttons, a row of FX keys, and not much else.

## Using it

![](docs/img/marconio.png)

The two left presets are locked to NTS Channel 1 and Channel 2 (live). The remaining four are yours — right-click to assign any NTS mixtape. Tap a preset to start listening. Tap the LCD to cycle through display themes (amber, blue, green, pink).

The FX row at the bottom lets you color the sound:

- **Clean** — straight through, no processing
- **Mag** — tape warble, gentle compression, a little warmth
- **Bass+** — low-end boost with some saturation
- **Radio** — mid-heavy, compressed, like a speaker in a dashboard

Keyboard shortcuts work the way you'd expect: `1`–`6` for presets, `Space` to play/stop, `Esc` to close menus.

On macOS and Windows, Marconio can live in your menu bar / system tray instead of the dock. The tray icon shows which preset you're listening to, and right-clicking it shows the current track info and a shortcut into settings.

Your preset assignments, display theme, and FX choice are remembered between sessions.

## What might not work

- **Stream drops** — if your connection hiccups, the audio stops but the UI might still look like it's playing until you hit stop/play again. There's no automatic reconnect yet. Just restart the app!
- **Linux** — haven't tried it at all, in theory it might work.
- **No volume knob** — volume is whatever your system volume is. In-app control isn't implemented yet.
- **Toolchain quirks on macOS** — if you're building from source and hit a `libclang` architecture mismatch error, it's a Rust/Xcode toolchain issue, not a Marconio bug.

## Building from source

You'll need Rust (stable), Deno 2.x, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform. Tool versions are pinned in `mise.toml` if you use [mise](https://mise.jdx.dev).

```bash
# install dependencies
mise install && mise run setup
# or: npm install

# run in development
mise run dev
# or: npm run tauri dev

# run tests
mise run test
# or: npm test

# build distributable
mise run bundle
# or: npm run tauri build
```
