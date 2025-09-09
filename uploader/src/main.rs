mod serial;

use std::{fs::File, io::Write, path::PathBuf, time::Duration};

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use serialport::SerialPort;
use thiserror::Error;

#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Dumps the EEPROM data
    Read {
        #[arg(short, long)]
        out_file: PathBuf,

        #[arg(short, long, default_value_t = 0x0000)]
        start: u16,

        #[arg(short, long, default_value_t = 0x8000)]
        end: u16,
    },

    /// Writes a file to the EEPROM
    Write {
        /// Binary file to upload
        filename: PathBuf,
    },

    /// Verifies the EEPROM's contents
    Verify {
        /// Binary file to verify against
        filename: PathBuf,
    },
}

impl Command {
    fn code(&self) -> u8 {
        match self {
            Self::Read { .. } => 0,
            Self::Write { .. } => 1,
            Self::Verify { .. } => 2,
        }
    }
}

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

#[derive(Debug, Error)]
enum Error {
    #[error("Could not open serial port: {0}")]
    SerialPort(#[from] serialport::Error),

    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Board sent an invalid opcode: {0}")]
    InvalidOpcode(u8),

    #[error("Received packet is not applicable to current state")]
    InvalidPacketStateCombo,

    #[error("Unknown: {0}")]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
enum Packet {
    Ready,
    Print(String),
    Chunk(Vec<u8>),
    ReadEnd,
}

fn read_packet(port: &mut Box<dyn SerialPort>) -> Result<Packet, Error> {
    let opcode = serial::read_u8(port)?;

    match opcode {
        0x00 => Ok(Packet::Ready),
        0x01 => {
            let len = serial::read_u16(port)?.into();
            let bytes = serial::read_n_bytes(port, len)?;
            let str = String::from_utf8(bytes).map_err(|e| anyhow!(e))?;
            Ok(Packet::Print(str))
        }
        0x02 => {
            let len = serial::read_u8(port)?.into();
            let bytes = serial::read_n_bytes(port, len)?;
            Ok(Packet::Chunk(bytes))
        }
        0x03 => Ok(Packet::ReadEnd),
        _ => Err(Error::InvalidOpcode(opcode)),
    }
}

#[derive(Debug)]
enum State {
    Readying,
    Reading {
        progress: usize,
        total: usize,
        out_file: File,
    },
}

const CHUNK_ACK: u8 = 0xFF;

fn run() -> Result<(), Error> {
    let args = Args::parse();

    println!("Opening port...");
    let mut port = serialport::new(args.port, args.baud_rate)
        .timeout(Duration::from_millis(args.timeout))
        .open()?;

    println!("Listening to packets...");

    let mut state = State::Readying;

    loop {
        let packet = read_packet(&mut port)?;

        match (&mut state, packet) {
            (_, Packet::Ready) => {
                port.write_all(&[args.command.code()])?;

                match &args.command {
                    Command::Read {
                        out_file: out_path,
                        start,
                        end,
                    } => {
                        println!("Initiating EEPROM read...");
                        state = State::Reading {
                            progress: 0,
                            total: (end - start).into(),
                            out_file: File::create(out_path)?,
                        };

                        serial::write_u16(&mut port, *start)?;
                        serial::write_u16(&mut port, *end)?;
                    }
                    _ => todo!(),
                }
            }
            (_, Packet::Print(s)) => {
                print!("{}", s);
                std::io::stdout().flush()?;
            }
            (
                State::Reading {
                    progress,
                    total,
                    out_file,
                },
                Packet::Chunk(new_data),
            ) => {
                *progress += new_data.len();
                out_file.write_all(&new_data)?;
                port.write_all(&[CHUNK_ACK])?;

                print!("\rProgress: {}%", (*progress * 100) / *total);
                std::io::stdout().flush()?;
            }
            (State::Reading { .. }, Packet::ReadEnd) => {
                println!();
                return Ok(());
            }
            _ => return Err(Error::InvalidPacketStateCombo),
        };
    }
}

fn main() -> anyhow::Result<()> {
    Ok(run()?)
}
