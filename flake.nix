{
  description = "Tesla Auth — Tesla API token generator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Runtime dependencies for WebKitGTK-based WebView
        buildInputs = [
          pkgs.webkitgtk_4_1
          pkgs.xdotool
          pkgs.gtk3
          pkgs.cairo
          pkgs.pango
          pkgs.gdk-pixbuf
          pkgs.glib
          pkgs.libsoup_3
          pkgs.openssl
          pkgs.glib-networking # TLS backend for HTTPS in WebKitGTK
          pkgs.nss # Certificates and crypto for WebKit
        ];

        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.wrapGAppsHook3 # Wraps binary with GIO/NSS env vars at install time
        ];
      in
      {
        # `nix build` → binary at result/bin/tesla_auth
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "tesla_auth";
          version = "0.13.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # Disable static_vcruntime build script (MSVC-only, irrelevant on Linux)
          STATIC_VCRUNTIME_NO_BUILD = "1";

          doCheck = false; # Tests require a graphical display (WebView)
        };

        # `nix develop` → development shell with Rust toolchain and all dependencies
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustc
            cargo
            clippy
            rustfmt
          ];

          inherit buildInputs;

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

          # TLS: glib-networking provides the GIO module for HTTPS in WebKitGTK
          GIO_EXTRA_MODULES = "${pkgs.glib-networking}/lib/gio/modules";

          # pkg-config must be able to locate the libraries
          PKG_CONFIG_PATH = with pkgs; lib.makeSearchPath "lib/pkgconfig" [
            webkitgtk_4_1
            gtk3
            libsoup_3
          ];
        };
      });
}
