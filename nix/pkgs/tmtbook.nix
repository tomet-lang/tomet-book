{
  craneLib,
  ...
}:
craneLib.buildPackage {
  src = craneLib.cleanCargoSource ../..;
  strictDeps = true;
}
