use std::io;

use derive_more::{Display, Error, From};
use serialport::SerialPort;

use crate::cobs;

mod host_op {
    pub const READ: u8 = 0;
    pub const WRITE: u8 = 1;
    pub const LOCK: u8 = 2;
    pub const UNLOCK: u8 = 3;
    pub const ERASE: u8 = 4;
    pub const EXIT: u8 = 255;
}

mod device_op {
    pub const ERR: u8 = 0;
    pub const READY: u8 = 1;
    pub const OK: u8 = 2;
    pub const BYTES: u8 = 3;
}

mod device_error {
    pub const PACKET_TOO_LONG: u8 = 0;
    pub const PACKET_EMPTY: u8 = 1;
    pub const INVALID_OP: u8 = 2;
    pub const MALFORMED_MESSAGE: u8 = 3;
    pub const WRITE_TIMEOUT: u8 = 4;
}

const PACKET_DELIM: u8 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
enum HostMessage {
    Exit,
    Read { start: u16, len: u8 },
    Write { start: u16, data: Vec<u8> },
    Lock,
    Unlock,
    Erase,
}

impl HostMessage {
    pub fn as_packet(&self) -> Vec<u8> {
        let mut packet = vec![];

        match self {
            Self::Exit => packet.push(host_op::EXIT),
            Self::Read { start: from, len } => {
                packet.push(host_op::READ);
                packet.extend_from_slice(&from.to_le_bytes());
                packet.push(*len);
            }
            Self::Write { start: from, data } => {
                packet.push(host_op::WRITE);
                packet.extend_from_slice(&from.to_le_bytes());
                packet.extend_from_slice(data);
            }
            Self::Lock => packet.push(host_op::LOCK),
            Self::Unlock => packet.push(host_op::UNLOCK),
            Self::Erase => packet.push(host_op::ERASE),
        }

        packet
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum DeviceError {
    #[display("Device received a too long packet")]
    PacketTooLong,
    #[display("Device received an empty packet")]
    PacketEmpty,
    #[display("Device received an invalid opcode")]
    InvalidOp,
    #[display("Device received a malformed message")]
    MalformedMessage,
    #[display("A write operation timed out")]
    WriteTimeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum DeviceMessage {
    Err(DeviceError),
    Ready,
    Ok,
    #[display("Bytes(...)")]
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Error)]
pub enum ParseDeviceMessageError {
    #[display("Host received an invalid error code: {_0}")]
    InvalidErr(#[error(ignore)] u8),

    #[display("Host received a malformed packet: {_0:?}")]
    MalformedPacket(#[error(ignore)] Vec<u8>),
}

impl TryFrom<&[u8]> for DeviceMessage {
    type Error = ParseDeviceMessageError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let message = match data {
            &[device_op::ERR, e] => {
                let err = match e {
                    device_error::PACKET_TOO_LONG => DeviceError::PacketTooLong,
                    device_error::PACKET_EMPTY => DeviceError::PacketEmpty,
                    device_error::INVALID_OP => DeviceError::InvalidOp,
                    device_error::MALFORMED_MESSAGE => DeviceError::MalformedMessage,
                    device_error::WRITE_TIMEOUT => DeviceError::WriteTimeout,
                    e => return Err(ParseDeviceMessageError::InvalidErr(e)),
                };

                Ok(DeviceMessage::Err(err))
            }
            &[device_op::READY] => Ok(DeviceMessage::Ready),
            &[device_op::OK] => Ok(DeviceMessage::Ok),
            [device_op::BYTES, data @ ..] => Ok(DeviceMessage::Bytes(data.to_vec())),
            _ => Err(ParseDeviceMessageError::MalformedPacket(data.to_vec())),
        }?;

        Ok(message)
    }
}

#[derive(Debug, From, Display, Error)]
pub enum ClientError {
    AlreadyExited,
    UnexpectedResponse(#[error(ignore)] DeviceMessage),
    ParseDeviceMessage(ParseDeviceMessageError),
    Io(io::Error),
    SerialPort(serialport::Error),
}

pub type ClientResult<T> = Result<T, ClientError>;

#[derive(Debug)]
pub struct Client {
    port: Box<dyn SerialPort>,
    exited: bool,
}

impl Client {
    pub fn new(port: Box<dyn SerialPort>) -> ClientResult<Self> {
        let mut client = Self {
            port,
            exited: false,
        };

        client.wait_for_ready()?;
        Ok(client)
    }

    pub fn exit(&mut self) -> ClientResult<()> {
        self.send_message(&HostMessage::Exit)?;
        self.recv_ok()?;
        self.exited = true;
        Ok(())
    }

    pub fn read(&mut self, start: u16, len: u8) -> ClientResult<Vec<u8>> {
        self.send_message(&HostMessage::Read { start, len })?;

        let resp = self.recv_message()?;
        let DeviceMessage::Bytes(data) = resp else {
            return Err(ClientError::UnexpectedResponse(resp));
        };

        Ok(data)
    }

    pub fn write(&mut self, start: u16, data: &[u8]) -> ClientResult<()> {
        self.send_message(&HostMessage::Write {
            data: data.to_vec(),
            start,
        })?;

        self.recv_ok()?;
        Ok(())
    }

    pub fn lock(&mut self) -> ClientResult<()> {
        self.send_message(&HostMessage::Lock)?;
        self.recv_ok()?;
        Ok(())
    }

    pub fn unlock(&mut self) -> ClientResult<()> {
        self.send_message(&HostMessage::Unlock)?;
        self.recv_ok()?;
        Ok(())
    }

    pub fn erase(&mut self) -> ClientResult<()> {
        self.send_message(&HostMessage::Erase)?;
        self.recv_ok()?;
        Ok(())
    }

    fn recv_ok(&mut self) -> ClientResult<()> {
        let resp = self.recv_message()?;
        if resp != DeviceMessage::Ok {
            return Err(ClientError::UnexpectedResponse(resp));
        }

        Ok(())
    }

    fn send_packet(&mut self, data: &[u8]) -> ClientResult<()> {
        self.check_not_exited()?;

        let encoded_data = cobs::encode(data, PACKET_DELIM);
        self.port.write_all(&encoded_data)?;
        self.port.write_all(&[PACKET_DELIM])?;
        Ok(())
    }

    fn send_message(&mut self, message: &HostMessage) -> ClientResult<()> {
        let packet = message.as_packet();
        self.send_packet(&packet)?;
        Ok(())
    }

    fn recv_message(&mut self) -> ClientResult<DeviceMessage> {
        let packet = self.recv_packet()?;
        let message = DeviceMessage::try_from(packet.as_slice())?;
        Ok(message)
    }

    fn recv_packet(&mut self) -> io::Result<Vec<u8>> {
        let mut encoded = vec![];

        loop {
            let mut b = 0;
            self.port.read_exact(std::slice::from_mut(&mut b))?;
            if b == PACKET_DELIM {
                break;
            }

            encoded.push(b);
        }

        Ok(cobs::decode(&encoded, PACKET_DELIM))
    }

    fn wait_for_ready(&mut self) -> ClientResult<()> {
        self.port.clear(serialport::ClearBuffer::All)?;
        while self.recv_message()? != DeviceMessage::Ready {}
        Ok(())
    }

    fn check_not_exited(&self) -> ClientResult<()> {
        if self.exited {
            return Err(ClientError::AlreadyExited);
        }

        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if !self.exited
            && let Err(e) = self.exit()
        {
            eprintln!("Client did not exit successfully: {}", e);
        }
    }
}

pub struct BlockReader<'a> {
    client: &'a mut Client,
    start: u16,
    block_size: u8,
    data: Vec<u8>,
}

impl<'a> BlockReader<'a> {
    pub fn new(client: &'a mut Client, block_size: u8) -> Self {
        Self {
            client,
            start: 9,
            data: vec![],
            block_size,
        }
    }

    fn contains_addr(&self, addr: u16) -> bool {
        self.start <= addr && (addr as usize) < (self.start as usize) + self.data.len()
    }

    fn read_from_block(&self, addr: u16) -> Option<u8> {
        self.contains_addr(addr)
            .then(|| self.data[(addr - self.start) as usize])
    }

    fn load_block(&mut self, addr: u16) -> ClientResult<()> {
        self.data = self.client.read(addr, self.block_size)?;
        self.start = addr;
        Ok(())
    }

    pub fn read(&mut self, addr: u16) -> ClientResult<u8> {
        self.read_from_block(addr).map(Ok).unwrap_or_else(|| {
            self.load_block(addr)?;
            Ok(self.read_from_block(addr).unwrap())
        })
    }
}
