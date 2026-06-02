{ pkgs, ... }:

{
  # Language-specific package lists for project shells.
  # These should match attributes in the pinned nixpkgs used by the
  # flake.lock. If a package is missing on your channel, prefer the
  # pinned flake or replace with the correct attribute (e.g. some
  # channels expose `rust-analyzer` as `rust-analyzer-bin`).

  rustPackages = with pkgs; [
    rustc
    cargo
    rust-analyzer
  ];

  pythonPackages = with pkgs; [
    python3
    uv
  ];

  nodePackages = with pkgs; [
    nodejs
    yarn
    pnpm
  ];

}
