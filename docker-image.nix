{ dockerTools
, cctld
, lib
,
}:
dockerTools.buildLayeredImage {
  name = "ghcr.io/cspr-rad/cctl-rs";
  tag = "cctl-casper-node-2.0.0-rc4";
  extraCommands = ''
    mkdir -p tmp
  '';
  config = {
    Cmd = lib.getExe cctld;
    ExposedPorts = {
      # Node ports
      # PROTOCOL ports
      "11101/tcp" = { };
      "11102/tcp" = { };
      "11103/tcp" = { };
      "11104/tcp" = { };
      "11105/tcp" = { };

      # BINARY ports
      "12101/tcp" = { };
      "12102/tcp" = { };
      "12103/tcp" = { };
      "12104/tcp" = { };
      "12105/tcp" = { };

      # REST ports
      "13101/tcp" = { };
      "13102/tcp" = { };
      "13103/tcp" = { };
      "13104/tcp" = { };
      "13105/tcp" = { };

      # SSE ports
      "14101/tcp" = { };
      "14102/tcp" = { };
      "14103/tcp" = { };
      "14104/tcp" = { };
      "14105/tcp" = { };

      # Sidecar ports
      # NODE-CLIENT ports
      "12101/tcp" = { };
      "12102/tcp" = { };
      "12103/tcp" = { };
      "12104/tcp" = { };
      "12105/tcp" = { };

      # MAIN-RPC ports
      "21101/tcp" = { };
      "21102/tcp" = { };
      "21103/tcp" = { };
      "21104/tcp" = { };
      "21105/tcp" = { };

      # SPEC-EXEC ports
      "22101/tcp" = { };
      "22102/tcp" = { };
      "22103/tcp" = { };
      "22104/tcp" = { };
      "22105/tcp" = { };
    };
  };
}
