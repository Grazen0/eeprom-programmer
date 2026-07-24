use derive_more::{Display, Error, From};
use std::{io, num::TryFromIntError};

use crate::client::ClientError;

#[derive(Debug, Display, Error, From)]
pub enum CliError {
    Io(io::Error),
    Client(ClientError),
    TryFromInt(TryFromIntError),

    #[from(skip)]
    Clap(#[error(ignore)] String),

    #[display("No commands provided.")]
    NoCommandsProvided,

    #[from(skip)]
    #[display("Parse error at command #{_0}")]
    ParseError(#[error(ignore)] usize),

    #[from(skip)]
    #[display("Command #{_0} is empty")]
    EmptyCommand(#[error(ignore)] usize),

    #[from(skip)]
    #[display("Unknown command: \"{}\"", _0.escape_default())]
    UnknownCommand(#[error(ignore)] String),

    #[display("File is too large (got {got} bytes, must be at most {max})")]
    FileTooLarge {
        got: u64,
        max: u64,
    },

    #[display("Range out of bounds for EEPROM (0x{:04X} - 0x{:04X})", start, *start as u64 + len)]
    RangeOutOfBounds {
        start: u16,
        len: u64,
    },

    #[display("Failed to fix in {_0} attempts.")]
    FixFailed(#[error(ignore)] usize),
}

pub type CliResult<T> = Result<T, CliError>;
