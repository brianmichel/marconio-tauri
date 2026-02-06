# Tauri Native-Feel Checklist

Use this list to catch webview behaviors that make a desktop app feel like a website.

## Interaction and Input

- [x] Disable text selection for non-editable UI (`user-select: none`).
- [x] Keep selection enabled only for true editable fields.
- [x] Disable default browser context menu where not needed.
- [x] Block drag/drop navigation behavior in the webview.
- [x] Add keyboard controls that feel app-like (slot keys + transport controls).

## Browser Shortcut Leakage

- [x] Prevent global browser shortcuts on non-editable UI (`Cmd/Ctrl+A`, `Cmd/Ctrl+R`, zoom keys).
- [x] Prevent `F5`/`F12` behaviors in app runtime.

## Window and Branding

- [x] Remove template browser title and use product name.
- [x] Replace template favicon reference.

## Visual/Pointer Polish

- [x] Use drag-region cursor behavior (`grab`/`grabbing`) on draggable area.
- [x] Replace default browser focus ring with app-consistent `:focus-visible`.

## Notes for Future Passes

- [ ] Add platform-specific native menu shortcuts (e.g., About, Quit, standard Edit menu) if you want full OS-menu parity.
- [ ] Consider native tray/menu integration if background behavior is needed.
