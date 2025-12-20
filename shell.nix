{
  mkShell,

  eeprom-uploader,
  arduino-cli,
  rust-analyzer,
  xxd,
}:
mkShell {
  inputsFrom = [ eeprom-uploader ];

  packages = [
    arduino-cli
    rust-analyzer
    xxd
  ];
}
