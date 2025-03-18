{
  description = "Flake for development workflows.";

  inputs = {
    rainix.url = "github:rainprotocol/rainix";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, flake-utils, rainix }:
    flake-utils.lib.eachDefaultSystem (system: {
      packages = rainix.packages.${system};
      devShells = rainix.devShells.${system};
    });
}

# {
#   description = "Flake for development workflows.";

#   inputs = {
#     rainix.url = "github:rainprotocol/rainix";
#     flake-utils.url = "github:numtide/flake-utils";
#   };

#   outputs = { self, flake-utils, rainix }:
#     flake-utils.lib.eachDefaultSystem (system:
#     let
#         pkgs = rainix.pkgs.${system};
#       in

#      {
#       packages = rainix.packages.${system};
#       devShells = pkgs.mkShell {
#           packages = [
#             packages.raindex-prelude
#             packages.ob-rs-test
#             packages.rainix-wasm-artifacts
#             packages.rainix-wasm-test
#             packages.js-install
#             packages.build-js-bindings
#             packages.test-js-bindings
#             rain.defaultPackage.${system}
#             packages.ob-ui-components-prelude
#           ];

#           shellHook = rainix.devShells.${system}.default.shellHook;
#           buildInputs = rainix.devShells.${system}.default.buildInputs;
#           nativeBuildInputs = rainix.devShells.${system}.default.nativeBuildInputs;
#         };
#     });
# }
