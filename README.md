# tesla_auth

## Overview

## TODO

- [x] Error handling
- [x] Debug logs
- [x] Display simple HTML page with access and refresh token
- [x] Add GitHub Action
  - [x] Lint code
  - [ ] Build binaries for different operating systems and attach them to a release
- [ ] Create macOS app, Windows .exe etc.
  - [ ] Add icons
  - [ ] Show app in doc

## Platform-specific notes

All platforms uses [tao](https://github.com/rust-windowing/tao) to build the window, and wry re-export it as application module. Here are the underlying web engine each platform uses, and some dependencies you might need to install.

### Linux

tesla_auth uses [gtk-rs](https://gtk-rs.org/) and its related libraries for window creation and also needs [WebKitGTK](https://webkitgtk.org/) for WebView. So please make sure following packages are installed:

#### Arch Linux / Manjaro:

```bash
sudo pacman -S webkit2gtk libappindicator-gtk3
```

#### Debian / Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.0-dev libappindicator3-dev
```

#### Fedora

```bash
sudo dnf install gtk3-devel webkit2gtk3-devel libappindicator-gtk3-devel
```

### macOS

WebKit is native on macOS so everything should be fine.

### Windows

WebView2 provided by Microsoft Edge Chromium is used. So tesla_auth supports Windows 7, 8, and 10.

## License

Apache-2.0/MIT
