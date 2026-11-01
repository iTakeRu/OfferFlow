# OfferFlow

[![Release](https://img.shields.io/github/v/release/iTakeRu/OfferFlow)](https://github.com/iTakeRu/OfferFlow/releases/latest)
[![CI](https://github.com/iTakeRu/OfferFlow/actions/workflows/ci.yml/badge.svg)](https://github.com/iTakeRu/OfferFlow/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/github/license/iTakeRu/OfferFlow)](./LICENSE)

[简体中文](./README.md) | **English**

A Windows desktop app for managing your job search: track applications, archive job descriptions, record and review interviews, and organize resumes and experience entries. Supports Chinese and English interfaces and multiple themes, with job search records stored locally.

<!-- Screenshots: uncomment after adding files to docs/screenshots/
![Applications board](./docs/screenshots/board.png)
-->

## Features

- **Applications**: Kanban and list views, drag-and-drop stage changes, custom stages and job categories, and search and filters for company, role, city, category, and channel
- **Job snapshots**: Archive job links, saved web pages (.html / .mhtml), or pasted text for reference after a listing disappears; supports HTTP and headless browser capture, with an automatic background window fallback when content cannot be extracted
- **Job details**: Extract information from page structure or JobPosting structured data; optionally use Ollama or an OpenAI-compatible API for AI extraction
- **Interview reviews**: Track each interview round with recordings, audio annotations, and rich-text review notes
- **Dashboard**: Application breakdowns by stage and category, plus interviews scheduled for the next 14 days
- **Resume versions**: Import, preview, rename, and tag resumes
- **Experience library**: Organize work, education, projects, and other experience; share basic information across multiple description versions and compare them, copy individual fields, import JSON, and include entries in full backups
- **Data protection**: Deleted files go to a trash folder; optional incremental mirroring to another drive or a cloud-synced folder; automatic database copies before schema upgrades
- **Export and backup**: Excel export and full backup and restore

<!-- Screenshots: uncomment after adding files to docs/screenshots/
| Job snapshot | Interview review | Experience library |
|---|---|---|
| ![Job snapshot](./docs/screenshots/snapshot.png) | ![Interview review](./docs/screenshots/interview.png) | ![Experience library](./docs/screenshots/experience.png) |
-->

## Installation

[Download the latest release](https://github.com/iTakeRu/OfferFlow/releases/latest). v1.0.0 provides the four files below. Choose one of the first three; `SHA256SUMS.txt` is for verifying downloads. See Development below to build from source.

| Release file | Description |
|---|---|
| `OfferFlow_1.0.0_x64-setup.exe` | NSIS installer |
| `OfferFlow_1.0.0_x64_en-US.msi` | MSI installer |
| `OfferFlow_1.0.0_x64.exe` | Standalone executable; run it directly without renaming |
| `SHA256SUMS.txt` | SHA-256 checksums for the three files above |

**Requirements**

- Windows x64 with the [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) (included in Windows 11 and preinstalled on most Windows 10 systems)
- Microsoft Edge or Google Chrome installed locally for headless browser capture of job pages

**First launch**: The app is not code-signed yet, so Windows SmartScreen may show "Windows protected your PC". Click "More info → Run anyway" to continue. To verify a download, compare its hash with `SHA256SUMS.txt` on the release page:

```powershell
certutil -hashfile OfferFlow_1.0.0_x64-setup.exe SHA256
```

**Updates**: Settings → About OfferFlow shows the current version and offers a manual update check and a link to download releases. It only reads GitHub stable release information; downloading and installing are up to you.

## Quick start

1. Create an application with the company and role, or paste a job link to save a job snapshot
2. Drag cards across the board as the application moves through stages
3. Add interview rounds as they are scheduled, then attach recordings and write review notes afterward
4. Maintain resumes and experience entries, and copy what you need when applying

Use the language toggle to switch between Chinese and English, and the theme selector to change the appearance.

## Data location

The data folder is chosen in this order:

1. The folder set in the `OFFERFLOW_DATA_DIR` environment variable
2. An **existing** `data/` folder next to the executable
3. `%APPDATA%\offerflow`

- **Installed version**: data is stored in `%APPDATA%\offerflow` by default
- **Standalone executable**: create an empty `data/` folder beside the executable before the first launch to keep data alongside the app, so both can be moved together

Uninstalling does not remove your job search data from `%APPDATA%\offerflow`; delete that folder manually to remove it completely. To switch data folders, use full backup and restore to migrate your data.

## Data and privacy

- Application records, resumes, recordings, and job snapshots are stored locally, with backup, restore, and mirroring to a chosen folder
- AI extraction can use a local Ollama instance or a configured model endpoint; sending content to an external service requires confirmation showing the destination
- Cookies and API keys are encrypted using Windows DPAPI and excluded from exported backups
- Backup database integrity is checked before restoration, and the previous database is retained as a `.bak` file

## Development

Requires Node.js 22.18+, stable Rust, and the system dependencies for Tauri 2, including Visual Studio Build Tools on Windows.

```bash
npm ci
npm run tauri dev      # Development with hot reload
npm run tauri build    # Build the executable and installers
```

Build outputs are placed in `src-tauri/target/release/`:

| File | Description |
|---|---|
| `offerflow.exe` | Locally built executable, named `OfferFlow_1.0.0_x64.exe` in the release |
| `bundle/nsis/*-setup.exe` | NSIS installer |
| `bundle/msi/*.msi` | MSI installer |

Local checks, matching CI:

```bash
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Feedback

Report bugs or suggest features in [GitHub Issues](https://github.com/iTakeRu/OfferFlow/issues).

## Author and development tools

Developed by [@iTakeRu](https://github.com/iTakeRu), using Codex and Claude Code alternately to assist with implementation, code review, and cross-validation.

## License

[MIT](./LICENSE)
