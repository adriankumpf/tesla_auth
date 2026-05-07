{
  description = "tesla_auth";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs =
    {
      nixpkgs,
      ...
    }:
    let
      cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
      forAllSystems = with nixpkgs; (lib.genAttrs lib.systems.flakeExposed);
      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt);
      deps = pkgs: with pkgs; [
        glib
        pango
        atk
        webkitgtk_4_1
        libsoup_3
        xdotool
      ];
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = cargoToml.package.name;
            version = cargoToml.package.version;

            src = ./.;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [
              makeWrapper
              pkg-config
            ];

            buildInputs = deps pkgs;

            postInstall = ''
              wrapProgram $out/bin/tesla_auth \
                --set GIO_MODULE_DIR "${pkgs.glib-networking}/lib/gio/modules"
            '';

            meta = {
              description = cargoToml.package.description;
              homepage = "https://github.com/adriankumpf/tesla_auth";
              license = pkgs.lib.licenses.mit;
              mainProgram = "tesla_auth";
            };
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            name = "tesla_auth";

            buildInputs = with pkgs; [
              rustc
              rustfmt
              cargo
              rust-analyzer
              clippy
              cargo-watch
              pkg-config
              cargo-audit
              cargo-outdated
              lldb
            ] ++ (deps pkgs);

            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            GIO_MODULE_DIR = "${pkgs.glib-networking}/lib/gio/modules";
          };
        }
      );
      inherit formatter;
    };
}
