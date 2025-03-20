{
  description = "Flake for development workflows.";

  inputs = {
    # rainix.url = "github:rainprotocol/rainix";
    rainix.url =
      "github:0xgleb/rainix?rev=c056a98d9319642e541475242f18583337666c73";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, flake-utils, rainix }:
    flake-utils.lib.eachDefaultSystem (system: {
      packages = rainix.packages.${system};
      devShells = rainix.devShells.${system};
    });
}
