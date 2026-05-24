# cull

A minimal desktop app for sorting photos into date-based folders using EXIF metadata.

Drop photos in, pick a destination, done. Output structure: `YYYY/Month/DD/`.

---

## Download

Grab the latest binary from [Releases](https://github.com/harmeepatel/cull/releases).

| Platform | File |
|---|---|
| macOS (Apple Silicon) | `cull-macos-arm64.zip` |
| macOS (Intel) | `cull-macos-x64.zip` |
| Windows 64-bit | `cull-windows-x64.zip` |
| Windows 32-bit | `cull-windows-x86.zip` |
| Linux | `cull-linux-x64.tar.gz` |

---

## How it works

1. Choose a destination folder
2. Select **Copy** or **Move**
3. Drag & drop photos onto the window (or click to browse)
4. Photos are sorted into `<dest>/<Year>/<Month>/<Day>/`

Date is read from EXIF `DateTimeOriginal`. Falls back to file modification time if no EXIF data is present. Filename conflicts are resolved automatically.

## Supported formats

JPEG, PNG, WebP, HEIC/HEIF, TIFF, BMP, AVIF

RAW: RAF, ARW, CR2, CR3, NEF, NRW, DNG, ORF, RW2, PEF, RWL

---

## Build from source

**Prerequisites:** [Rust](https://rustup.rs) and the [Dioxus CLI](https://dioxuslabs.com/learn/0.6/getting_started)

```sh
cargo install dioxus-cli
git clone https://github.com/harmeepatel/cull
cd cull
dx serve --release
```

**Linux** also requires:
```sh
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```
