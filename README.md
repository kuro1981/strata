Devenv project template
========================

This directory contains a minimal template for creating a development shell for a project.

Files:

- `flake.nix` - Flakes-compatible devShell. Enter with `nix develop`.
- `shell.nix` - Legacy `nix-shell` entrypoint. Enter with `nix-shell`.

Usage (flake):

```bash
nix develop
```

Usage (legacy):

```bash
nix-shell
```

Customize the package list in `flake.nix` or `shell.nix` to match your project's needs.

Grouping development packages
----------------------------

You can keep grouped lists of packages (e.g. `rustPackages`, `pythonPackages`,
`nodePackages`) in a shared module like `modules/common/dev.nix` and then
compose the specific groups you need in your project's `flake.nix` or
`shell.nix`:

Example (flake import):

```nix
let pkgs = import nixpkgs { system = "x86_64-linux"; };
	devMod = import ../../modules/common/dev.nix { inherit pkgs; };
in pkgs.mkShell { buildInputs = devMod.commonDev ++ devMod.rustPackages; }
```

This lets you only enable Rust, Python, or JS sets per-project instead of
installing everything system-wide.

Per-language shells
-------------------

This template provides language-specific shells via flakes and legacy
`nix-shell` files.

- Flake attributes:
	- `nix develop .#default` — common dev tools
	- `nix develop .#rust` — common + Rust tools
	- `nix develop .#python` — common + Python tools
	- `nix develop .#node` — common + Node.js tools

- Legacy:
	- `nix-shell shell-rust.nix`
	- `nix-shell shell-python.nix`
	- `nix-shell shell-node.nix`

Pinning and distributing this template
-------------------------------------

This template includes a `flake.lock` that pins `nixpkgs`. To distribute
the template reproducibly, commit both `flake.nix` and `flake.lock` to your
repository. Consumers can then run `nix develop .#rust` (or another attr)
and get a deterministic environment.

Recommended steps to publish a project using this template:

1. Update `nixpkgs` when you want to refresh packages:

```bash
nix flake update --update-input nixpkgs
git add flake.lock
git commit -m "chore(dev): update nixpkgs pin"
```

2. Ensure `lang-dev.nix` package attributes match the pinned channel.
   If a package name differs, replace it in `lang-dev.nix` (e.g.
   `rust-analyzer` vs `rust-analyzer-bin`).

Enabling the `dev-home` sample for users
----------------------------------------

The repo includes a `home-templates/dev-home.nix` example that intentionally
leaves language toolchains disabled so users enable them per-project. To
enable the sample for a user, import it from your home configuration.

Example (in your `home`/home-manager config):

```nix
{ pkgs, ... }:

{
	imports = [ ./path/to/repo/nix-config/home-templates/dev-home.nix ];
}
```

After importing, edit `dev-home.nix` or your local override to include any
`dev` packages you want installed at the user level. The recommended flow
is to keep user-level packages minimal and enable language-specific toolchains
in the project `flake.nix`/`shell.nix` using the template's `lang-dev.nix`.

Project template helper script
------------------------------

The `init-devenv.sh` helper copies the template files into a target
project directory under a dedicated `.devenv/` folder so you can quickly
bootstrap a new repo without scattering template files in the project root.
Usage:

```bash
# copy template into /path/to/my-project/.devenv
./dev-templates/devenv/init-devenv.sh /path/to/my-project

# optionally choose a different folder name
./dev-templates/devenv/init-devenv.sh /path/to/my-project devenv

# then inside the template directory
cd /path/to/my-project/.devenv
nix develop .#default
```




