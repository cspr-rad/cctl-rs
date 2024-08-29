{ dockerTools
, cctld
, lib
,
}:
dockerTools.buildLayeredImage {
  name = "ghcr.io/cspr-rad/cctl-rs";
  tag = "cctl-casper-node-1.5.7";
  extraCommands = ''
    mkdir -p tmp
  '';
  config = {
    Cmd = lib.getExe cctld;
    ExposedPorts = {
      # RPC ports
      "11101/tcp" = { };
      "11102/tcp" = { };
      "11103/tcp" = { };
      "11104/tcp" = { };
      "11105/tcp" = { };
      # REST ports
      "14101/tcp" = { };
      "14102/tcp" = { };
      "14103/tcp" = { };
      "14104/tcp" = { };
      "14105/tcp" = { };
      # SSE ports
      "18101/tcp" = { };
      "18102/tcp" = { };
      "18103/tcp" = { };
      "18104/tcp" = { };
      "18105/tcp" = { };
      # Consensus ports
      "22101/tcp" = { };
      "22102/tcp" = { };
      "22103/tcp" = { };
      "22104/tcp" = { };
      "22105/tcp" = { };
    };
  };
}
