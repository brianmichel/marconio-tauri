# ShazamKit Manual Recognition Plan

Last updated: 2026-02-16
Owner: Codex + Brian
Scope: macOS manual song recognition for currently playing stream, persistent history, in-app history panel, and toast feedback.

## Decisions Locked
- Recognition mode: manual only (button-triggered).
- History persistence: yes, survives app restarts.
- History UI: in-app panel component (not separate window yet).
- Minimum macOS version: raise to 12.0.
- User feedback: toast in app on success/failure (OS notification optional follow-up).

## Status Legend
- `TODO`: not started
- `IN_PROGRESS`: currently being implemented
- `DONE`: completed and verified
- `BLOCKED`: waiting on external input/assets/access

## Todo Board
| ID | Task | Status | Notes |
|---|---|---|---|
| T1 | Add Objective-C ShazamKit bridge (C ABI) and compile from Tauri build | DONE | Added `src-tauri/src/shazam_bridge.h` + `src-tauri/src/shazam_bridge.m` and build wiring |
| T2 | Add Rust `ShazamManager` state and wire recognition lifecycle | DONE | Added `src-tauri/src/shazam.rs` with attempt state, timeout, and event flow |
| T3 | Feed decoded PCM frames from native playback worker into recognition pipeline | DONE | Added audio frame tap in `audio_engine.rs` and wired to `ShazamManager` |
| T4 | Add Tauri commands/events for identify/history/clear and result updates | DONE | Commands + event emissions wired in `lib.rs` and `shazam.rs` |
| T5 | Persist recognition history to app data JSON and load on startup | DONE | JSON persistence and load-on-start in `shazam.rs` |
| T6 | Add top-window "magic find" button UI and control states | DONE | Added controls in `App.vue` + styles in `receiver-shell.css` |
| T7 | Add reusable in-app recognized tracks panel component | DONE | Added `RecognizedTracksPanel.vue` and integrated with events/commands |
| T8 | Add reusable toast component for success/failure feedback | DONE | Added `ToastStack.vue` and hooked to result events |
| T9 | Update macOS minimum version to 12.0 and signing/capability config | IN_PROGRESS | `minimumSystemVersion` updated; Apple capability/profile still external |
| T10 | Add tests and manual QA checklist for success/no-match/error paths | IN_PROGRESS | frontend tests/typecheck passed; Rust compile checks blocked by local toolchain |
| T11 | Documentation updates (`README.md`) | DONE | added manual recognition usage and platform/capability note |

## Blockers / Inputs Needed
- Apple Developer configuration confirmation:
  - App ID capability enabled for ShazamKit.
  - Signing identity/profile includes required capability.
- Local Rust toolchain alignment:
  - `cargo check` currently blocked by `libclang` architecture mismatch (`stable-x86_64-apple-darwin` loading arm64 libclang).
  - `cargo fmt` unavailable because `rustfmt` is not installed for current toolchain.
- If we choose OS notifications now (optional): confirmation on desired notification style and copy.

## Rolling Update Log
- 2026-02-16: Plan document created with locked decisions and implementation todos.
- 2026-02-16: Started implementation; moved T1 to IN_PROGRESS.
- 2026-02-16: Completed T1-T8 and backend/frontend integration for manual recognition.
- 2026-02-16: Updated macOS minimum system version to 12.0; signing capability remains external input.
- 2026-02-16: Frontend test suite (`vitest`) and `vue-tsc --noEmit` passed.
- 2026-02-16: Rust verification blocked by local toolchain mismatch (`libclang` arch) and missing `rustfmt`.
- 2026-02-16: Updated README with song recognition usage and requirements.

## Change Protocol
- I will update this file whenever:
  - a todo changes status,
  - a blocker appears or is cleared,
  - implementation scope changes.

## QA Checklist (Manual)
- [ ] Start playback, press `FIND SONG`, observe `LISTENING...` state and eventual toast.
- [ ] Verify success path persists history and shows new row in `HITS` panel.
- [ ] Verify no-match path shows informational toast without history insert.
- [ ] Verify clear history removes saved items and persists empty state on restart.
- [ ] Verify app restart reloads recognized history from disk.
- [ ] Verify behavior on non-macOS (feature hidden/disabled and no crashes).
