{ tomet, ... }:
{
  projectRootFile = "flake.nix";
  programs = {
    #= Nix
    nixfmt.enable = true;
    statix.enable = true;
    deadnix.enable = true;

    #= Shell
    shfmt.enable = true;
    shellcheck.enable = true;

    #= Main
    rustfmt.enable = true;
    taplo.enable = true;
  };

  settings = {
    global.excludes = [
      "*.lock"
    ];

    formatter = {
      tomet = {
        command = "${tomet}/bin/tomet";
        options = [
          "format"
          "-i"
        ];
        includes = [ "*.tmt" ];
      };
    };

    shfmt = {
      includes = [ "*.sh" ];
    };

    biome = {
      includes = [
        "*.js"
        "*.ts"
        "*.jsx"
        "*.tsx"
        "*.json"
      ];
    };

    rustfmt = {
      includes = [ "*.rs" ];
    };
  };
}
