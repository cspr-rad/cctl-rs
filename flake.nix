{
  description = "cctl-rs";

  nixConfig = {
    extra-substituters = [
      "https://crane.cachix.org"
      "https://nix-community.cachix.org"
      "https://casper-cache.marijan.pro"
      "https://cspr.cachix.org"
    ];
    extra-trusted-public-keys = [
      "crane.cachix.org-1:8Scfpmn9w+hGdXH/Q9tTLiYAE/2dnJYRJP7kl80GuRk="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "casper-cache.marijan.pro:XIDjpzFQTEuWbnRu47IqSOy6IqyZlunVGvukNROL850="
      "cspr.cachix.org-1:vEZlmbOsmTXkmEi4DSdqNVyq25VPNpmSm6qCs4IuTgE="
    ];
  };

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
    advisory-db.url = "github:rustsec/advisory-db";
    advisory-db.flake = false;
    # with casper-node 2.0.0-rc4 https://github.com/casper-network/cctl/pull/44/commits/058de6887c691dac0b20284fba1f78bcd39187ef
    cctl.url = "github:cspr-rad/cctl/use-client-feat-2.0";
    csprpkgs.follows = "cctl/csprpkgs";
  };

  outputs = inputs@{ flake-parts, treefmt-nix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      imports = [
        treefmt-nix.flakeModule
        ./nixos
        ./dummy-contract
      ];
      perSystem = { self', inputs', pkgs, lib, ... }:
        let
          rustToolchain = inputs'.fenix.packages.stable.toolchain;
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

          cctl = inputs'.cctl.packages.cctl;

          cctlAttrs = {
            pname = "cctl-rs";

            src = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.lock
                ./bin
                ./src
                ./tests
                ./test-resources
              ];
            };

            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = with pkgs; [
              openssl.dev
            ] ++ lib.optionals stdenv.isDarwin [
              libiconv
              darwin.apple_sdk.frameworks.Security
              darwin.apple_sdk.frameworks.SystemConfiguration
            ];

            # the coverage report will run the tests
            doCheck = false;

            checkInputs = [
              cctl
            ];
          };
        in
        {
          devShells.default = pkgs.mkShell {
            inputsFrom = [ self'.packages.cctld ];
            packages = [ cctl ];
          };

          packages = {
            cctl-rs-deps = craneLib.buildDepsOnly cctlAttrs;

            cctl-rs-docs = craneLib.cargoDoc (cctlAttrs // {
              cargoArtifacts = self'.packages.cctl-rs-deps;
            });

            cctld = craneLib.buildPackage (cctlAttrs // {
              pname = "cctld";
              cargoArtifacts = self'.packages.cctl-rs-deps;

              nativeBuildInputs = cctlAttrs.nativeBuildInputs ++ [
                pkgs.makeWrapper
              ];

              postInstall = ''
                wrapProgram $out/bin/cctld \
                  --set PATH ${pkgs.lib.makeBinPath [ cctl ]}
              '';

              meta.mainProgram = "cctld";
            });

            default = self'.packages.cctld;

            cctl-docker-image = pkgs.callPackage ./docker-image.nix { inherit (self'.packages) cctld; };
          };

          checks = {
            lint = craneLib.cargoClippy (cctlAttrs // {
              pname = "cctl-rs-lint";
              cargoArtifacts = self'.packages.cctl-rs-deps;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

            coverage-report = craneLib.cargoTarpaulin (cctlAttrs // {
              pname = "cctl-rs-coverage-report";
              cargoArtifacts = self'.packages.cctl-rs-deps;
              # Default values from https://crane.dev/API.html?highlight=tarpau#cranelibcargotarpaulin
              # --avoid-cfg-tarpaulin fixes nom/bitvec issue https://github.com/xd009642/tarpaulin/issues/756#issuecomment-838769320
              cargoTarpaulinExtraArgs = "--skip-clean --out xml --output-dir $out --avoid-cfg-tarpaulin";
              # cargoTarpaulin runs the tests in the buildPhase
              buildInputs = cctlAttrs.buildInputs ++ [
                cctl
              ];
            });
          };

          treefmt = {
            projectRootFile = ".git/config";
            programs.nixpkgs-fmt.enable = true;
            programs.rustfmt.enable = true;
            programs.rustfmt.package = craneLib.rustfmt;
            settings.formatter = { };
          };
        };
      flake = {
        herculesCI.ciSystems = [ "x86_64-linux" ];
      };
    };
}
