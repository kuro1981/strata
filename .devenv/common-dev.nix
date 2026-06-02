{ pkgs, ... }:

{
  # Common development tools intended for per-project shells (devenv).
  # This file only declares a package list (`commonDev`) and should be
  # imported by `flake.nix` or `shell.nix` in the project. Keep this list
  # minimal: language-specific toolchains belong in `lang-dev.nix` or in
  # per-project flakes.
  #
  # Example usage in `shell.nix`:
  # let pkgs = import <nixpkgs> {}; dev = import ./common-dev.nix { inherit pkgs; }; in
  # pkgs.mkShell { buildInputs = dev.commonDev; }

  commonDev = with pkgs; [
    gh
    neovim
    fzf
    direnv
    devenv
    bat
    tmux
    sd
    ripgrep
    fd
  ];

}
