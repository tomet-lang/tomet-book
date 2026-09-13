{
  craneLib,
  lib,
}:
import ./tmtbook.nix {
  inherit craneLib lib;
  features = [ "embedded-search" ];
}
