{
  mkShell,

  eeprom-uploader,
  arduino-cli,
  xxd,
}:
mkShell {
  inputsFrom = [ eeprom-uploader ];

  packages = [
    arduino-cli
    xxd
  ];
}
