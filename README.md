# Tesla Auth

[![CI](https://github.com/adriankumpf/tesla_auth/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/adriankumpf/tesla_auth/actions/workflows/ci.yml)
[![CD](https://github.com/adriankumpf/tesla_auth/actions/workflows/cd.yml/badge.svg?branch=main)](https://github.com/adriankumpf/tesla_auth/actions/workflows/cd.yml)

Securely generate API tokens for third-party access to your Tesla.

Multi-factor authentication (MFA) and Captcha are supported.

## Download

> _Precompiled binaries are currently only available for x86-64._

- [macOS](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla-auth-macos.tar.gz)
- [Linux](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla-auth-linux.tar.gz)
- [Windows](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla-auth-windows.tar.gz)

## Usage

```plain
❯ tesla_auth --help
Usage: tesla_auth [-d]

Tesla API tokens generator

Options:
  -d, --debug       print debug output
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

[WebKitGTK](https://webkitgtk.org/) is required for WebView. So please make sure the following packages are installed:

#### Arch Linux / Manjaro:

```bash
sudo pacman -S webkit2gtk-4.1
```

#### Debian / Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev
```

#### Fedora

```bash
sudo dnf install gtk3-devel webkit2gtk4.1-devel
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

On Linux you'll need to install the [required dev dependencies](https://github.com/adriankumpf/tesla_auth/blob/main/.github/workflows/cd.yml#L47) first.

## License

Apache-2.0/MIT
