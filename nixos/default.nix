{ self, inputs, ... }:
{
  flake = {
    checks."x86_64-linux" =
      let pkgs = inputs.nixpkgs.legacyPackages."x86_64-linux";
      in
      {
        verify-cctl-service =
          pkgs.callPackage
            ./tests/verify-cctl-service.nix
            {
              inherit (inputs.csprpkgs.packages.${pkgs.system}) casper-client-rs;
              cctlModule = self.nixosModules.cctl;
              contractWasm = ../test-resources/demo-contract-optimized.wasm;
            };
      };
    nixosModules = {
      cctl =
        { pkgs, ... }:
        {
          imports = [ ./modules/cctl.nix ];
          services.cctl.package = self.packages.${pkgs.system}.cctld;
        };
    };
  };
}
