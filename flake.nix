{
  description = "SquidMod - Modding Tool for Splatoon on Cemu/Xapfish";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    {
      overlays.default = final: prev: {
        squidmod = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };

      nixosModules = {
        default = self.nixosModules.squidmod;
        squidmod = { config, lib, pkgs, ... }:
          let
            cfg = config.programs.squidmod;
          in
          {
            options.programs.squidmod = {
              enable = lib.mkEnableOption "SquidMod, a modding tool for Splatoon on Cemu/Xapfish";
            };

            config = lib.mkIf cfg.enable {
              environment.systemPackages = [
                self.packages.${pkgs.system}.default
              ];
            };
          };
      };
    }
    // flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        squidmod = pkgs.rustPlatform.buildRustPackage {
          pname = "squidmod";
          version = "9.0.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
            copyDesktopItems
          ];

          buildInputs = with pkgs; [
            gtk4
            libadwaita
            glib
            openssl
          ];

          postInstall = ''
            install -Dm644 assets/hicolor/256x256/apps/dev.jerrysm64.squidmod.png \
              $out/share/icons/hicolor/256x256/apps/dev.jerrysm64.squidmod.png
          '';

          desktopItems = [
            (pkgs.makeDesktopItem {
              name = "SquidMod";
              exec = "SquidMod";
              icon = "dev.jerrysm64.squidmod";
              comment = "Modding Tool for Splatoon on Cemu/Xapfish";
              desktopName = "SquidMod";
              categories = [ "Game" "Utility" ];
            })
          ];

          meta = with pkgs.lib; {
            description = "Modding Tool for Splatoon on Cemu/Xapfish";
            homepage = "https://github.com/JerrySM64/SquidMod";
            license = licenses.mit;
            mainProgram = "SquidMod";
            platforms = platforms.linux;
          };
        };
      in
      {
        packages = {
          default = squidmod;
          squidmod = squidmod;
        };

        apps = {
          default = flake-utils.lib.mkApp {
            drv = squidmod;
            exePath = "/bin/SquidMod";
          };
          squidmod = self.apps.${system}.default;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ squidmod ];
          packages = with pkgs; [
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt
          ];
        };
      }
    );
}
