{
  description = "lyre — a terminal music library & player";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Native build inputs required by lyre's dependencies:
        #   alsa-lib  — rodio audio backend on Linux
        #   dbus      — zbus D-Bus / MPRIS integration
        #   pkg-config — lets the build scripts find the above
        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs = with pkgs; [
          alsa-lib
          dbus
        ];

      in {
        # ── packages.default — the lyre binary ──────────────────────────────
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "lyre";
          version = "0.1.0";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          inherit nativeBuildInputs buildInputs;

          # Tell pkg-config where to find alsa and dbus at build time
          PKG_CONFIG_PATH = with pkgs; lib.makeSearchPath "lib/pkgconfig" [
            alsa-lib.dev
            dbus.dev
          ];

          meta = with pkgs.lib; {
            description  = "A terminal music library and player";
            homepage     = "https://github.com/rkliman/lyre";
            license      = licenses.mit;
            maintainers  = [ "Randall Kliman" ];
            mainProgram  = "lyre";
            platforms    = platforms.linux;
          };
        };

        # ── apps.default — runnable via `nix run` ───────────────────────────
        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };

        # ── devShells.default — `nix develop` drops you into a build env ───
        devShells.default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt
          ]);

          # Ensure pkg-config can locate alsa and dbus during `cargo build`
          PKG_CONFIG_PATH = with pkgs; lib.makeSearchPath "lib/pkgconfig" [
            alsa-lib.dev
            dbus.dev
          ];

          shellHook = ''
            echo "lyre dev shell — rust $(rustc --version)"
          '';
        };
      }
    );
}
