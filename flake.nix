{
  description = "Flake for development workflows.";

  inputs = {
    # rainix.url = "github:rainprotocol/rainix";
    rainix.url =
      "github:0xgleb/rainix?rev=ac41a9b7643e20d2bcd04b390c71cc5f8ddaecfb";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, flake-utils, rainix }:
    flake-utils.lib.eachDefaultSystem (system: {
      packages = rainix.packages.${system};
      devShells = rainix.devShells.${system};
    });
}
