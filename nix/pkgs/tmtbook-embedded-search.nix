{
  craneLib,
  lib,
  tailwindcss,
  esbuild,
}:
import ./tmtbook.nix {
  inherit
    craneLib
    lib
    tailwindcss
    esbuild
    ;
  features = [ "embedded-search" ];
}
