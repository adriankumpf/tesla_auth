# Tesla Auth

[![CI](https://github.com/adriankumpf/tesla_auth/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/adriankumpf/tesla_auth/actions/workflows/ci.yml)
[![CD](https://github.com/adriankumpf/tesla_auth/actions/workflows/release.yml/badge.svg)](https://github.com/adriankumpf/tesla_auth/actions/workflows/release.yml)

Securely generate API tokens for third-party access to your Tesla.

Multi-factor authentication (MFA) and Captcha are supported.

## Download

- macOS [Apple Silicon](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla_auth-aarch64-apple-darwin.tar.xz) / [Intel](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla_auth-x86_64-apple-darwin.tar.xz)
- [Linux](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla_auth-x86_64-unknown-linux-gnu.tar.xz)
- [Windows](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla_auth-x86_64-pc-windows-msvc.zip)

## Usage

```plain
❯ tesla_auth --help
Usage: tesla_auth [-d] [-k]

Tesla API tokens generator

Options:
  -d, --debug       print debug output
  -c, --clear-browsing-data
                    clear browsing data at startup
  --help            display usage information
```

### Steps

1. Run the `tesla_auth` executable (either by double-clicking it or directly in a terminal)
2. Enter your Tesla account credentials (and MFA code if necessary)
3. You'll get a final window where you can select and copy the access token and refresh token

## Platform-specific dependencies

### macOS

WebKit is native on macOS so no additional dependencies are required.

### Windows

WebView2 provided by Microsoft Edge Chromium is used. So Windows 7, 8, 10 and 11 are supported.

### Linux

[WebKitGTK](https://webkitgtk.org/) is required for WebView and `libxdo` is used to make the predfined Copy, Cut, Paste and SelectAll menu items work. So please make sure the following packages are installed:

#### Arch Linux / Manjaro:

```bash
sudo pacman -S webkit2gtk-4.1 xdotool
```

#### Debian / Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev libxdo-dev
```

#### Fedora

```bash
sudo dnf install gtk3-devel webkit2gtk4.1-devel xdotool
```

## Development

```bash
# Clone repository
git clone https://github.com/adriankumpf/tesla_auth
cd tesla_auth

# Build (debug version)
cargo build

# Install (release version)
cargo install --path . --locked
```

## License

Apache-2.0/MIT
