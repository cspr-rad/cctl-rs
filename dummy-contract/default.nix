{ inputs, ... }:
{
  perSystem = { self', inputs', pkgs, lib, ... }:
    let
      # nightly-2023-03-25: https://github.com/casper-network/casper-node/blob/release-2.0.0-rc4/smart_contracts/rust-toolchain
      toolchainAttrs = { channel = "nightly"; date = "2023-03-25"; sha256 = "sha256-vWMW7tpbU/KseRztVYQ6CukrQWJgPgtlFuB6OPoZ/v8="; };
      rustToolchain = with inputs'.fenix.packages; combine [
        (toolchainOf toolchainAttrs).toolchain
        (targets.wasm32-unknown-unknown.toolchainOf toolchainAttrs).rust-std
      ];
      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

      contractAttrs = {
        pname = "dummy-contract";
        src = lib.cleanSourceWith {
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./src
            ];
          };
        };
        cargoExtraArgs = "--target wasm32-unknown-unknown";
        CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "lld";
        nativeBuildInputs = [ pkgs.binaryen pkgs.llvmPackages_16.bintools ];
        doCheck = false;
        # optimize wasm
        postInstall = ''
          directory="$out/bin/"
          for file in "$directory"*.wasm; do
            if [ -e "$file" ]; then
              wasm-opt -Oz --strip-debug --signext-lowering "$file"
            fi
          done
        '';
      };
    in
    {
      devShells.contract = pkgs.mkShell {
        inputsFrom = [ self'.packages.dummy-contract ];
        CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "lld";
      };
      packages = {
        # Used for testing purposes
        dummy-contract = craneLib.buildPackage contractAttrs;
      };
    };
  flake = { };
}
