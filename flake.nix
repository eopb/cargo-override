{
  description = "Cargo subcommand for overriding dependencies";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    {
      overlays.default = (
        final: prev: {
          cargo-override = self.packages.${final.system}.cargo-override;
        }
      );
    }
    // (flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        darwinDeps =
          with pkgs;
          lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Foundation
            libiconv
          ];

      in
      {
        packages = {
          cargo-override = pkgs.rustPlatform.buildRustPackage {
            pname = "cargo-override";
            version = "unstable-${self.shortRev or "dirty"}";

            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
            buildInputs =
              with pkgs;
              [
                libssh2
              ]
              ++ darwinDeps;

            preCheck = ''
              export RUST_BACKTRACE=1
            '';

            checkFlags = [
              # Broken when run under the nix sandbox
              "--skip=cli_tests::override_subcommand_help_message"
              "--skip=git::git_patch"
              "--skip=git::git_patch_branch"
              "--skip=git::git_patch_branch"
              "--skip=git::git_patch_rev"
              "--skip=git::git_patch_tag"
              "--skip=git::git_patch_tag"
              "--skip=git::git_patch_version_missmatch"
              "--skip=git::git_patch_version_missmatch"
              "--skip=missing_manifest"
              "--skip=patch_absolute_path"
              "--skip=patch_manifest_doesnt_exist"
            ];

          };
          default = self.packages.${system}.cargo-override;
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.cargo-override}/bin/cargo-override";
        };

        formatter = pkgs.nixfmt-rfc-style;

        checks.cargo-override = self.packages.${system}.cargo-override.overrideAttrs (
          { ... }:
          {
            buildPhase = "true";
            installPhase = "touch $out";
            cargoCheckType = "test";
          }
        );

        devShells.default = pkgs.mkShell {
          buildInputs =
            with pkgs;
            [
              # Dependencies
              curl
              pkg-config

              # Additional tools recommended by contributing.md
              cargo-insta
              cargo-nextest
              nil
            ]
            ++ darwinDeps;

          shellHook = ''
            export RUST_BACKTRACE=1
          '';
        };
      }
    ));
}
