# EEPROM Programmer

Code to program an EEPROM using an Arduino MEGA.

## Usage

Some usage examples:

```bash
# Unlock, write a file, and lock again
eeprom-programmer \
    'unlock' \
    'write <file.bin>' \
    'lock'

# Read the EEPROM's first 256 bytes into out.bin
eeprom-programmer 'read out.bin --len=256'

# Doom
eeprom-programmer 'erase'
```

The CLI supports the following commands:

- `read <FILE> [--start=<ADDR>] [--len=<NUM>]`: Dumps the EEPROM's contents to `<FILE>`.
- `write <FILE> [--no-verify]`: Writes `<FILE>` to the EEPROM starting at address 0.
- `verify <FILE> [--fix]`: Verifies the EEPROM against `<FILE>`.
- `unlock`: Executes the software data protection (SDP) disable algorithm.
- `lock`: Executes the SDP enable algorithm.
- `erase`: Executes the software chip erase algorithm.

## References

- [AT28C256 Datasheet](https://ww1.microchip.com/downloads/aemDocuments/documents/MPD/ProductDocuments/DataSheets/AT28C256-Industrial-Grade-256-Kbit-Paged-Parallel-EEPROM-Data-Sheet-DS20006386.pdf)
- [Software Chip Erase](http://ww1.microchip.com/downloads/en/Appnotes/doc0544.pdf)
