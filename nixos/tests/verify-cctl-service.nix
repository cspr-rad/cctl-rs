{ nixosTest
, casper-client-rs
, cctlModule
, contractWasm
}:
nixosTest {
  name = "verify-cctl-service";

  nodes = {
    server = { config, ... }: {
      virtualisation.diskSize = 2048;
      imports = [
        cctlModule
      ];
      services.cctl = {
        enable = true;
        contract = { "contract-hash" = contractWasm; };
      };
      networking.firewall.allowedTCPPorts = [ 80 config.services.cctl.port ];
    };
    client = { pkgs, ... }: {
      environment.systemPackages = [ pkgs.wget casper-client-rs ];
    };
  };

  testScript = { nodes }:
    let
      casperNodeAddress = "http://server:${builtins.toString nodes.server.services.cctl.port}";
      # This is the directory wget will copy to, see script below
      clientUsersDirectory = "server/cctl/users";
    in
    ''
      start_all()
      server.wait_for_unit("cctl.service")

      # verify that the cctl network is running and reached the validate state
      response = client.succeed("casper-client get-peers --node-address ${casperNodeAddress}")

      # FIXME: getting the status is currently broken beteen sidecar <-> node
      # import json

      # response = client.succeed("casper-client get-node-status --node-address ${casperNodeAddress}")
      # response_json = json.loads(response)
      # assert "result" in response_json and "Validate" in response_json["result"].get("reactor_state"), "The node didn't reach the VALIDATE state. The status response was {}".format(response)

      # verify that the generated cctl test accounts are being served
      client.succeed("wget --no-parent -r http://server/cctl/users/")
      client.succeed("cat ${clientUsersDirectory}/user-1/public_key_hex")
    '';
}
