with import <nixpkgs> {};

# Minimal project shell: imports the shared `commonDev` list and exposes a
# simple environment. For language-specific shells (Rust/Python/Node) use
# the separate `shell-rust.nix`, `shell-python.nix`, `shell-node.nix`, or
# the flake devShell attributes (e.g. `nix develop .#rust`).

let dev = import ./.devenv/common-dev.nix { inherit pkgs; };
in pkgs.mkShell {
  buildInputs = dev.commonDev;

  # Add a friendly shell prompt hint and a basic shellHook example.
  shellHook = ''
    echo "Entering project dev shell (common tools)."
  '';
}
