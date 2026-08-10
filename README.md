# Tesla Auth

[![CI](https://github.com/adriankumpf/tesla_auth/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/adriankumpf/tesla_auth/actions/workflows/ci.yml)
[![CD](https://github.com/adriankumpf/tesla_auth/actions/workflows/release.yml/badge.svg)](https://github.com/adriankumpf/tesla_auth/actions/workflows/release.yml)

Securely generate API tokens for third-party access to your Tesla.

Supports MFA and Captcha through Tesla's native login flow.

## Download

- macOS [Apple Silicon](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla_auth-aarch64-apple-darwin.tar.xz) / [Intel](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla_auth-x86_64-apple-darwin.tar.xz)
- Linux [x86_64](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla_auth-x86_64-unknown-linux-gnu.tar.xz) / [ARM](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla_auth-aarch64-unknown-linux-gnu.tar.xz)
- [Windows](https://github.com/adriankumpf/tesla_auth/releases/latest/download/tesla_auth-x86_64-pc-windows-msvc.zip)

## Usage

```plain
❯ tesla_auth --help
Usage: tesla_auth [-d] [-c] [-v]

Tesla API tokens generator

Options:
  -d, --debug       print debug output
  -c, --clear-browsing-data
                    clear browsing data at startup
  -v, --version     print the version and exit
  --help, help      display usage information
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

[WebKitGTK](https://webkitgtk.org/) 4.1 is required for WebView and `libxdo` is used to make the predfined Copy, Cut, Paste and SelectAll menu items work. Ubuntu 22.04, Debian 12 and Fedora 36 are the earliest releases that ship WebKitGTK 4.1; on anything older neither the prebuilt binaries nor a local build will run.

So please make sure the following packages are installed:

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

## Troubleshooting

Run `tesla_auth --debug` first: it prints the URLs the webview navigates to, which is usually enough to tell where a flow gets stuck.

### Blank window, or a window that disappears immediately (Linux)

WebKitGTK's accelerated rendering paths misbehave on a number of drivers, the NVIDIA proprietary one in particular. Typical symptoms are `Failed to create GBM buffer of size …` or `Error 71 (Protocol error) dispatching to Wayland display`.

`tesla_auth` therefore disables the DMA-BUF renderer by default. If the window still does not come up, try:

```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 tesla_auth   # turn off compositing entirely
GDK_BACKEND=x11 tesla_auth                     # run under XWayland
WEBKIT_DISABLE_DMABUF_RENDERER=0 tesla_auth    # opt back into the default renderer
```

### The login form keeps returning to the sign-in page

Stale cookies from a previous session are the usual cause. Start over with a clean profile:

```bash
tesla_auth --clear-browsing-data
```

### `Access Denied — You don't have permission to access … on this server`

The request was rejected by Tesla's CDN before it ever reached the login page (the reference URL points at `errors.edgesuite.net`, i.e. Akamai). This is an IP reputation block rather than something `tesla_auth` can influence — disconnect from a VPN, or force a new public IP by power-cycling your router, and try again.

### TeslaMate reports `Error: Tokens are invalid`

The tokens are pasted into TeslaMate, which then talks to `auth.tesla.com` itself. If that request fails, TeslaMate reports the tokens as invalid even though they are fine — check its logs for the actual error, and retry once Tesla's SSO endpoints are healthy again.

### macOS logs `_TIPropertyValueIsValid called with 11 on nil context!`

Noise from the system input manager that any WebKit-based app produces. It has no effect on the login flow.

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

MIT
