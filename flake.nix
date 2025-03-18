{
  description = "Flake for development workflows.";

  inputs = {
    rainix.url = "github:rainprotocol/rainix";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, flake-utils, rainix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        rust-version = "1.79.0";
        rust-toolchain =
          pkgs.rust-bin.stable.${rust-version}.default.override ({
            extensions = [ "rust-src" ];
          });

        pkgs = rainix.pkgs.${system};
      in {
        packages = rainix.packages.${system};
        devShell = pkgs.mkShell {
          packages = with pkgs; [ rust-analyzer nixfmt-classic ];

          shellHook = rainix.devShells.${system}.default.shellHook;
          buildInputs = [ rust-toolchain ]
            ++ rainix.devShells.${system}.default.buildInputs;
          nativeBuildInputs =
            rainix.devShells.${system}.default.nativeBuildInputs;
        };
      });
}
