# Provisioning Profiles

This directory stores CI-only provisioning profiles referenced by Tauri config overlays:

- `ci-development-marconio.provisionprofile`
- `ci-distribution-marconio.provisionprofile`

The base local development config (`src-tauri/tauri.conf.json`) does not reference these files.

Local development tasks in `mise.toml` reference `src-tauri/tauri.ci-development.conf.json`, so `mise run dev`, `mise run build`, and `mise run bundle` expect `ci-development-marconio.provisionprofile` to exist.

CI/release overlays:

- `src-tauri/tauri.ci-development.conf.json`
- `src-tauri/tauri.ci-distribution.conf.json`
