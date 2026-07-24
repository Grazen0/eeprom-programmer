use derive_more::{Display, Error, From};
use std::{io, num::TryFromIntError};

use crate::client::ClientError;

#[derive(Debug, Display, Error, From)]
#[from(io::Error, ClientError, TryFromIntError)]
pub enum CliError {
    #[from]
    Io(io::Error),

    #[from]
    Client(ClientError),

    #[from]
    TryFromInt(TryFromIntError),

    Clap(#[error(ignore)] String),

    #[display("No commands provided.")]
    NoCommandsProvided,

    #[display("Parse error at command #{_0}")]
    ParseError(#[error(ignore)] usize),

    #[display("Command #{_0} is empty")]
    EmptyCommand(#[error(ignore)] usize),

    #[display("Unknown command: {:?}", _0)]
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

    #[display("Verification found {_0} mismatches.")]
    VerifyFailed(#[error(ignore)] usize),
}

impl From<clap::Error> for CliError {
    fn from(e: clap::Error) -> Self {
        Self::Clap(e.to_string())
    }
}

pub type CliResult<T> = Result<T, CliError>;
