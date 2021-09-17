# tesla_auth

## Platform-specific dependencies

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

WebKit is native on macOS so no additional dependencies are required.

### Windows (not tested!)

WebView2 is powered by Microsoft Edge (Chromium). So tesla_auth should support Windows 7, 8, and 10.

## License

Apache-2.0/MIT
