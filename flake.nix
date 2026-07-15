{
  description = "Devenv project flake template";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    llm-agents.url = "github:numtide/llm-agents.nix";
  };

  outputs = { self, nixpkgs, llm-agents, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      # Prefer a local `.devenv` folder when present in the project
      useLocalTemplate = builtins.pathExists ./.devenv;

      devCommon = if useLocalTemplate
        then import ./.devenv/common-dev.nix { inherit pkgs; llm-agents = llm-agents.packages.${system}; }
        else import ./common-dev.nix { inherit pkgs; llm-agents = llm-agents.packages.${system}; };

      devLang = if useLocalTemplate
        then import ./.devenv/lang-dev.nix { inherit pkgs; }
        else import ./lang-dev.nix { inherit pkgs; };
    in {
      packages.${system} = {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "strata-cli";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
        };
      };

      devShells.${system} = {
        # Primary, default development shell. `init-devenv.sh` will
        # adjust the `buildInputs` line here to include language
        # packages if you choose one when bootstrapping the project.
        default = pkgs.mkShell {
          # G1 WS-B (docs/graph-ui-g1-handoff.md, sml-spec.md §1.13 D50): ui/ の
          # pnpm ワークスペースをビルドするため Node.js(LTS)+ pnpm を追加。
          # devLang.nodePackages(.devenv/lang-dev.nix)は nodejs/yarn/pnpm を持つが
          # yarn は使わないので個別に足す(commonDev/rustPackages と同じ粒度)。
          buildInputs = devCommon.commonDev ++ devLang.rustPackages ++ [
            pkgs.typst
            pkgs.hackgen-font
            pkgs.noto-fonts-cjk-sans
            pkgs.ipaexfont
            pkgs.nodejs
            pkgs.pnpm
          ];
          shellHook = ''
            export PATH="$HOME/.local/bin:$PATH"
            export TYPST_FONT_PATHS="${pkgs.hackgen-font}/share/fonts:${pkgs.noto-fonts-cjk-sans}/share/fonts:${pkgs.ipaexfont}/share/fonts"
          '';
        };

        # Example additional shells (kept commented as examples):
        #
        # rust = pkgs.mkShell {
        #   buildInputs = devCommon.commonDev ++ devLang.rustPackages;
        # };
        #
        # python = pkgs.mkShell {
        #   buildInputs = devCommon.commonDev ++ devLang.pythonPackages;
        # };
        #
        # node = pkgs.mkShell {
        #   buildInputs = devCommon.commonDev ++ devLang.nodePackages;
        # };
      };
    };

}
