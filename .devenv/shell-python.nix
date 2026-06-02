let
  pkgs = import <nixpkgs> {};
  devMod = import ./.devenv/lang-dev.nix { inherit pkgs; };
  devCommon = import ./.devenv/common-dev.nix { inherit pkgs; };
  devLang = import ./lang-dev.nix { inherit pkgs; };
in pkgs.mkShell {
  buildInputs = devCommon.commonDev ++ devLang.pythonPackages;
}
