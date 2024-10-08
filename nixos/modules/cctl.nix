{ lib, config, ... }:
let
  inherit (lib)
    types
    mkOption
    mkIf
    mkMerge
    mkEnableOption
    escapeShellArgs
    optionals
    ;
  cfg = config.services.cctl;
in
{
  options.services.cctl = {

    enable = mkEnableOption "cctl";

    package = mkOption {
      type = types.package;
    };

    port = mkOption {
      type = types.port;
      default = 21101;
      example = 60000;
      description = ''
        Port to listen on.
        TODO make port configurable in cctl
      '';
    };

    workingDirectory = mkOption {
      type = types.path;
      default = "/var/lib/cctl";
      description = ''
        The working directory path where cctl will put its assets and resources.
      '';
    };

    chainspec = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = ''
        The path to a chainspec.toml.
      '';
    };

    config = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = ''
        The path to a casper node config.toml.
      '';
    };

    logLevel = mkOption {
      type = types.str;
      default = "info";
      description = ''
        The log-level that should be used.
      '';
    };

    contracts = mkOption {
      default = null;
      type = types.nullOr (types.listOf (types.submodule {
        options = {
          hash_name = mkOption {
            type = types.str;
            description = ''
              The contracts hash name which was provided when calling
              https://docs.rs/casper-contract/latest/casper_contract/contract_api/storage/fn.new_locked_contract.html
            '';
          };
          path = mkOption {
            type = types.path;
            description = "The wasm compiled contract that should be deployed once the network is up and ready.";
          };
          runtime_args = mkOption {
            default = null;
            type = types.nullOr types.attrs;
            description = "The runtime arguments expected by this contract.";
          };
        };
      }));
    };
  };

  config = mkIf cfg.enable {

    systemd.services.cctl =
      let
        args = escapeShellArgs ([
          "--working-dir"
          cfg.workingDirectory
        ]
        ++ optionals (!builtins.isNull cfg.contracts) ([
          "--deploy-contracts"
        ] ++ (lib.map (contract: builtins.toJSON contract) cfg.contracts))
        ++ optionals (!builtins.isNull cfg.chainspec) [
          "--chainspec-path"
          cfg.chainspec
        ]
        ++ optionals (!builtins.isNull cfg.config) [
          "--config-path"
          cfg.config
        ]);
      in
      {
        description = "cctl";
        documentation = [ "" ];
        wantedBy = [ "multi-user.target" ];
        after = [ "network-online.target" ];
        requires = [ "network-online.target" ];
        environment = {
          RUST_LOG = cfg.logLevel;
        };
        serviceConfig =
          mkMerge [
            {
              ExecStart = "${lib.getExe cfg.package} ${args}";
              Type = "notify";
              Restart = "no";
              User = "cctl";
              Group = "cctl";
              TimeoutStartSec = 1000;
              StateDirectory = builtins.baseNameOf cfg.workingDirectory;
              WorkingDirectory = cfg.workingDirectory;
              ReadWritePaths = [
                cfg.workingDirectory
              ];
            }
          ];
      };

    users.users.cctl = {
      name = "cctl";
      group = "cctl";
      isSystemUser = true;
    };
    users.groups.cctl = { };

    # Allows nginx to have read access to the working directory of cctl
    users.users.${config.services.nginx.user}.extraGroups = [ "cctl" ];

    # Since cctl is usually ran on the same machine as the application that is subject to be tested,
    # we need to serve the generated users directory to make it available for client machines
    # when testing
    services.nginx = {
      enable = true;
      virtualHosts."${config.networking.hostName}".locations = {
        "/cctl/users/" = {
          alias = "${cfg.workingDirectory}/assets/users/";
          extraConfig = ''
            autoindex on;
            add_header Content-Type 'text/plain charset=UTF-8';
          '';
        };
        "/cctl/contracts/" = {
          alias = "${cfg.workingDirectory}/contracts/";
          extraConfig = ''
            autoindex on;
            add_header Content-Type 'text/plain charset=UTF-8';
          '';
        };
      };
    };
  };
}
