mod error;
mod protocol;
mod serial;

use std::{fs::File, io::Write, path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};
use derive_more::Display;

use crate::{
    error::Error,
    protocol::Packet,
    serial::{Serial, SerialPortSerial},
};

#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Dumps the EEPROM data to a file
    Read {
        #[arg(short, long)]
        out_file: PathBuf,

        #[arg(short, long, default_value_t = 0x0000)]
        start: u16,

        #[arg(short, long, default_value_t = 0x8000)]
        end: u16,
    },

    /// Writes a file to the EEPROM
    Write { filename: PathBuf },

    /// Verifies the EEPROM's data against a file
    Verify {
        filename: PathBuf,
        #[arg(long)]
        fix: bool,
    },
}

/// A program to interact with AT28C EEPROM chips
#[derive(Debug, Clone, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Port where the Arduino is connected
    #[arg(short, long)]
    port: String,

    /// Baud rate used by the Arduino
    #[arg(short, long, default_value_t = 115200)]
    baud_rate: u32,

    /// Timeout (in milliseconds) for connecting to the Arduino
    #[arg(short, long, default_value_t = 10)]
    timeout: u64,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Display)]
enum State {
    #[display("Readying")]
    Readying,

    #[display("Reading")]
    Reading {
        progress: usize,
        total: usize,
        out_file: File,
        out_path: PathBuf,
    },

    #[display("Writing")]
    Writing { current_byte: usize, data: Vec<u8> },

    #[display("Verifying")]
    Verifying { current_byte: usize, data: Vec<u8> },

    #[display("Finished")]
    Finished(Result<(), Error>),
}

const CHUNK_ACK: u8 = 0xFF;

fn state_transition(
    state: State,
    packet: Packet,
    port: &mut impl Serial,
    args: &Args,
) -> Result<State, Error> {
    let next_state = match (state, packet) {
        (_, Packet::Ready) => match args.command {
            Command::Read {
                out_file: ref out_path,
                start,
                end,
            } => {
                if end < start {
                    return Err(Error::InvalidRegionBounds);
                }

                println!("Initiating EEPROM read...");

                port.write_u8(0x00)?;
                port.write_u16(start)?;
                port.write_u16(end)?;

                State::Reading {
                    progress: 0,
                    total: (end - start).into(),
                    out_file: File::create(out_path)?,
                    out_path: out_path.clone(),
                }
            }
            Command::Write { ref filename } => {
                println!("Initiating EEPROM write...");

                let data = std::fs::read(filename)?;

                port.write_u8(0x01)?;

                State::Writing {
                    current_byte: 0,
                    data,
                }
            }
            Command::Verify { ref filename, fix } => {
                println!("Initiating EEPROM verification...");

                let data = std::fs::read(filename)?;

                port.write_u8(0x02)?;
                port.write_u8(fix.into())?;

                State::Verifying {
                    current_byte: 0,
                    data,
                }
            }
        },
        (state, Packet::Print(s)) => {
            print!("{}", s);
            std::io::stdout().flush()?;
            state
        }
        (_, Packet::InvalidChecksum { expected, computed }) => {
            State::Finished(Err(Error::InvalidPacketChecksum { expected, computed }))
        }

        (
            State::Reading {
                progress,
                total,
                mut out_file,
                out_path,
            },
            Packet::Chunk {
                data: chunk_data,
                checksum,
            },
        ) => {
            assert_eq!(
                checksum,
                protocol::calculate_checksum(&chunk_data),
                "chunk checksum comparison failed"
            );

            let new_progress = progress + chunk_data.len();
            out_file.write_all(&chunk_data)?;

            port.write_u8(CHUNK_ACK)?;

            print!("\rProgress: {}%", (new_progress * 100) / total);
            std::io::stdout().flush()?;

            State::Reading {
                progress: new_progress,
                total,
                out_file,
                out_path,
            }
        }
        (State::Reading { out_path, .. }, Packet::ReadEnd) => {
            println!();
            println!("Memory contents successfully dumped to {:?}", out_path);
            State::Finished(Ok(()))
        }

        (State::Writing { current_byte, data }, Packet::ChunkRequest)
            if current_byte >= data.len() =>
        {
            println!();
            port.write_u8(0x00)?;
            State::Finished(Ok(()))
        }
        (
            State::Writing {
                mut current_byte,
                data,
            },
            Packet::ChunkRequest,
        ) => {
            protocol::send_data_chunk(port, &data, &mut current_byte)?;
            State::Writing { current_byte, data }
        }

        (
            state @ State::Verifying { .. },
            Packet::ByteMismatch {
                address,
                expected,
                computed,
            },
        ) => {
            println!(
                "Byte mismatch at 0x{:04X} (expected = 0x{:02X}, was = 0x{:02X})",
                address, expected, computed
            );
            state
        }
        (State::Verifying { current_byte, data }, Packet::ChunkRequest)
            if current_byte >= data.len() =>
        {
            println!();
            port.write_u8(0x00)?;
            State::Finished(Ok(()))
        }
        (
            State::Verifying {
                mut current_byte,
                data,
            },
            Packet::ChunkRequest,
        ) => {
            protocol::send_data_chunk(&mut *port, &data, &mut current_byte)?;
            State::Verifying { current_byte, data }
        }

        (state, packet) => State::Finished(Err(Error::UnexpectedPacket {
            state_variant: state.to_string(),
            packet,
        })),
    };

    Ok(next_state)
}

fn run(args: Args) -> Result<(), Error> {
    println!("Opening port...");
    let mut port = SerialPortSerial::new(
        &args.port,
        args.baud_rate,
        Duration::from_millis(args.timeout),
    )?;

    let mut state = State::Readying;

    loop {
        let packet = protocol::read_packet(&mut port)?;
        state = state_transition(state, packet, &mut port, &args)?;

        if let State::Finished(result) = state {
            return result;
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    Ok(run(args)?)
}
