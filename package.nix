{ lib
, rustPlatform
, cctl
, makeWrapper
, pkg-config
, openssl
, stdenv
, darwin
, ...
}:
rustPlatform.buildRustPackage {
  pname = "cctl-rs";
  version = "0.0.1";
  src = ./cctl-rs;
  cargoHash = "sha256-p+N2/T+iJF6waTrHYn5J6F/PP/fsU7Ypcxyh9AvEqBA=";

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];

  buildInputs = [
    openssl
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];

  postInstall = ''
    wrapProgram $out/bin/cctld \
      --set PATH ${lib.makeBinPath [ cctl ]}
  '';

  nativeCheckInputs = [ cctl ];

  meta.mainProgram = "cctld";
  meta.license = lib.licenses.mit;
}
