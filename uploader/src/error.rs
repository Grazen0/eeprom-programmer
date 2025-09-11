use derive_more::{Display, Error, From};

use crate::protocol::{Packet, ProtocolError};

#[derive(Debug, From, Display, Error)]
pub enum Error {
    #[display("Could not open serial port: {_0}")]
    SerialPort(#[from] serialport::Error),

    #[display("I/O error: {_0}")]
    IO(#[from] std::io::Error),

    Protocol(#[from] ProtocolError),

    #[display("Board sent an invalid opcode: {_0}")]
    InvalidOpcode(#[error(not(source))] u8),

    #[display("Received an unexpected packet (state = {state_variant}, packet = {packet})")]
    UnexpectedPacket {
        state_variant: String,
        packet: Packet,
    },

    #[display(
        "A packet did not arrive with the expected checksum. (expected = 0x{expected:02X}, computed = 0x{computed:02X})"
    )]
    InvalidPacketChecksum {
        expected: u16,
        computed: u16,
    },

    #[display("Memory region bounds must be valid")]
    InvalidRegionBounds,

    #[display("Unknown: {_0}")]
    Unknown(#[from] anyhow::Error),
}
