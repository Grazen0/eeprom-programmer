use std::io;

use derive_more::{Display, Error, From};
use serialport::SerialPort;

use crate::cobs;

mod host_op {
    pub const EXIT: u8 = 0;
    pub const READ: u8 = 1;
    pub const WRITE: u8 = 2;
    pub const LOCK: u8 = 3;
    pub const UNLOCK: u8 = 4;
}

mod dev_op {
    pub const ERR: u8 = 0;
    pub const READY: u8 = 1;
    pub const OK: u8 = 2;
    pub const BYTES: u8 = 3;
}

mod dev_error {
    pub const PACKET_TOO_LONG: u8 = 0;
    pub const PACKET_EMPTY: u8 = 1;
    pub const INVALID_OP: u8 = 2;
    pub const MALFORMED_MESSAGE: u8 = 3;
}

const PACKET_DELIM: u8 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
enum HostMessage {
    Exit,
    Read { start: u16, len: u8 },
    Write { start: u16, data: Vec<u8> },
    Lock,
    Unlock,
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
        }

        packet
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum DevError {
    #[display("Device received a too long packet")]
    PacketTooLong,
    #[display("Device received an empty packet")]
    PacketEmpty,
    #[display("Device received an invalid host opcode")]
    InvalidOp,
    #[display("Device received a malformed message")]
    MalformedMessage,
}

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum DevMessage {
    Err(DevError),
    Ready,
    Ok,
    #[display("Bytes(...)")]
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Error)]
pub enum ParseDevMessageError {
    #[display("Host received an empty packet")]
    PacketEmpty,

    #[display("Host received an invalid error code ({_0})")]
    InvalidErr(#[error(ignore)] u8),

    #[display("Host received an invalid opcode ({_0})")]
    InvalidOp(#[error(ignore)] u8),

    #[display("Host received a malformed message")]
    MalformedMessage,
}

impl TryFrom<&[u8]> for DevMessage {
    type Error = ParseDevMessageError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let mut iter = data.iter();
        let op = iter
            .next()
            .copied()
            .ok_or(ParseDevMessageError::PacketEmpty)?;

        let message = match data {
            [dev_op::ERR] => {
                let err = match iter.next() {
                    Some(&dev_error::PACKET_TOO_LONG) => DevError::PacketTooLong,
                    Some(&dev_error::PACKET_EMPTY) => DevError::PacketEmpty,
                    Some(&dev_error::INVALID_OP) => DevError::InvalidOp,
                    Some(&dev_error::MALFORMED_MESSAGE) => DevError::MalformedMessage,
                    Some(&e) => return Err(ParseDevMessageError::InvalidErr(e)),
                    None => return Err(ParseDevMessageError::MalformedMessage),
                };

                Ok(DevMessage::Err(err))
            }
            [dev_op::READY] => Ok(DevMessage::Ready),
            [dev_op::OK] => Ok(DevMessage::Ok),
            [dev_op::BYTES, data @ ..] => Ok(DevMessage::Bytes(data.to_vec())),
            _ => Err(ParseDevMessageError::InvalidOp(op)),
        }?;

        Ok(message)
    }
}

#[derive(Debug, From, Display, Error)]
pub enum ClientError {
    AlreadyExited,
    UnexpectedResponse(#[error(ignore)] DevMessage),
    ParseDevMessage(ParseDevMessageError),
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

    pub fn lock_eeprom(&mut self) -> ClientResult<()> {
        self.send_message(&HostMessage::Lock)?;
        self.recv_ok()?;
        Ok(())
    }

    pub fn unlock_eeprom(&mut self) -> ClientResult<()> {
        self.send_message(&HostMessage::Unlock)?;
        self.recv_ok()?;
        Ok(())
    }

    pub fn read_data(&mut self, start: u16, len: u8) -> ClientResult<Vec<u8>> {
        self.send_message(&HostMessage::Read { start, len })?;

        let resp = self.recv_message()?;
        let DevMessage::Bytes(data) = resp else {
            return Err(ClientError::UnexpectedResponse(resp));
        };

        Ok(data)
    }

    pub fn write_data(&mut self, data: &[u8], start: u16) -> ClientResult<()> {
        self.send_message(&HostMessage::Write {
            data: data.to_vec(),
            start,
        })?;

        self.recv_ok()?;
        Ok(())
    }

    fn recv_ok(&mut self) -> ClientResult<()> {
        let resp = self.recv_message()?;
        if resp != DevMessage::Ok {
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

    fn recv_message(&mut self) -> ClientResult<DevMessage> {
        let packet = self.recv_packet()?;
        let message = DevMessage::try_from(packet.as_slice())?;
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
        while self.recv_message()? != DevMessage::Ready {}
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
            eprintln!("Client did not exit successfully: {:?}", e);
        }
    }
}
