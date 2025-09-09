# EEPROM Programmer

Code to program an EEPROM using an Arduino MEGA.

## Protocol

Data is sent from the Arduino to the uploader in **packets**. A packet begins
with an **opcode** and may or may not include additional **parameters**
depending on the opcode.

The following table shows the different packets the Arduino may send to the
uploader:

|   Name    |                      Description                      | Opcode |          Parameters           |
| :-------: | :---------------------------------------------------: | :----: | :---------------------------: |
|  `Ready`  |   Signals that the board is ready to receive data.    | `0x00` |                               |
|  `Print`  |           Prints a string to the terminal.            | `0x01` | `size: u16, str: [u8; size]`  |
|  `Chunk`  | An incoming data chunk when using the `read` command. | `0x02` | `size: u16, data: [u8; size]` |
| `ReadEnd` |     Signals that the `read` command has finished.     | `0x03` |                               |

> [!NOTE]
> Parameters of type `u16` are sent in big-endian.
