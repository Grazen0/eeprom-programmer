use std::string::FromUtf8Error;

use derive_more::{Display, Error, From};

use crate::serial::SerialIO;

#[derive(Debug, From, Display, Error)]
pub enum ProtocolError {
    #[display("Received a packet with invalid opcode: {_0:02X}")]
    InvalidPacketOpcode(#[error(not(source))] u8),

    #[display("A received string packet does not contain valid UTF-8")]
    InvalidUtf8(#[from] FromUtf8Error),

    #[display("Unknown error: {_0}")]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Display)]
pub enum Packet {
    #[display("Ready")]
    Ready,
    #[display("Print")]
    Print(String),
    #[display("Chunk")]
    Chunk { data: Vec<u8>, checksum: u16 },
    #[display("ReadEnd")]
    ReadEnd,
    #[display("ChunkRequest")]
    ChunkRequest,
    #[display("InvalidChecksum")]
    InvalidChecksum { expected: u16, computed: u16 },
    #[display("ByteMismatch")]
    ByteMismatch {
        address: u16,
        expected: u8,
        found: u8,
    },
    #[display("ByteRequest")]
    ByteRequest,
}

pub fn calculate_checksum(data: &[u8]) -> u16 {
    let mut sum_1 = 0_u8;
    let mut sum_2 = 0_u8;

    for &n in data {
        sum_1 = sum_1.wrapping_add(n);
        sum_2 = sum_2.wrapping_add(sum_1);
    }

    u16::from_ne_bytes([sum_1, sum_2])
}

pub fn read_packet(port: &mut dyn SerialIO) -> Result<Packet, ProtocolError> {
    let opcode = port.read_u8()?;

    match opcode {
        0x00 => Ok(Packet::Ready),
        0x01 => {
            let len = port.read_u16()?.into();
            let bytes = port.read_n(len)?;
            let str = String::from_utf8(bytes)?;
            Ok(Packet::Print(str))
        }
        0x02 => {
            let len = port.read_u8()?.into();
            let checksum = port.read_u16()?;
            let data = port.read_n(len)?;
            Ok(Packet::Chunk { data, checksum })
        }
        0x03 => Ok(Packet::ReadEnd),
        0x04 => Ok(Packet::ChunkRequest),
        0x05 => {
            let expected = port.read_u16()?;
            let computed = port.read_u16()?;
            Ok(Packet::InvalidChecksum { expected, computed })
        }
        0x06 => {
            let address = port.read_u16()?;
            let expected = port.read_u8()?;
            let computed = port.read_u8()?;
            Ok(Packet::ByteMismatch {
                address,
                expected,
                found: computed,
            })
        }
        0x07 => Ok(Packet::ByteRequest),
        _ => Err(ProtocolError::InvalidPacketOpcode(opcode)),
    }
}

pub fn send_data_chunk(
    port: &mut impl SerialIO,
    data: &[u8],
    current_byte: &mut usize,
) -> anyhow::Result<()> {
    const CHUNK_MAX_SIZE: usize = 16;
    let data_left = &data[*current_byte..];

    let chunk = &data_left[..CHUNK_MAX_SIZE.min(data_left.len())];

    port.write_u8(chunk.len().try_into().unwrap())?;
    port.write_u16(calculate_checksum(chunk))?;
    port.write_n(chunk)?;
    *current_byte += chunk.len();

    Ok(())
}
