mod core;
mod protocol;
mod serial;

use std::{io::Write, path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};

use crate::{
    core::{Effect, Error, State, UserCommand, UserOptions},
    serial::SerialPortIO,
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
    Write {
        filename: PathBuf,

        #[arg(long)]
        no_verify: bool,
    },

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
    /// Path to the port where the board is connected
    #[arg(short, long, default_value = "/dev/ttyUSB0")]
    port: String,

    /// Baud rate for the connection
    #[arg(short, long, default_value_t = 115200)]
    baud_rate: u32,

    /// Timeout (in milliseconds) for connecting to the Arduino
    #[arg(short, long, default_value_t = 10)]
    timeout: u64,

    #[command(subcommand)]
    command: Command,
}

impl From<Args> for UserOptions {
    fn from(args: Args) -> Self {
        Self {
            command: match args.command {
                Command::Read {
                    out_file,
                    start,
                    end,
                } => UserCommand::Read {
                    out_filename: out_file,
                    start,
                    end,
                },
                Command::Write {
                    filename,
                    no_verify,
                } => UserCommand::Write {
                    in_filename: filename,
                    verify: !no_verify,
                },
                Command::Verify { filename, fix } => UserCommand::Verify {
                    in_filename: filename,
                    fix,
                },
            },
        }
    }
}

const BAR_LEN: usize = 20;

fn handle_effect(effect: Effect) -> std::io::Result<()> {
    match effect {
        Effect::PrintLn(s) => println!("{}", s),
        Effect::Print(s) => {
            print!("{}", s);
            std::io::stdout().flush()?;
        }
        Effect::Progress { done, total } => {
            let filled = (done * BAR_LEN) / total;
            let empty = BAR_LEN - filled;

            print!(
                "\rProgress: [{}{}] {}%",
                "#".repeat(filled),
                ".".repeat(empty),
                (done * 100) / total
            );
            std::io::stdout().flush()?;
        }
        Effect::VerifyProgress {
            done,
            total,
            mismatches,
        } => {
            let filled = (done * BAR_LEN) / total;
            let empty = BAR_LEN - filled;

            print!(
                "\rProgress: [{}{}] {}%, mismatches: {}",
                "#".repeat(filled),
                ".".repeat(empty),
                (done * 100) / total,
                mismatches
            );
            std::io::stdout().flush()?;
        }
        Effect::ProgressEnd => println!(),
    }

    Ok(())
}

fn run(args: Args) -> Result<(), Error> {
    println!("Opening serial port...");

    let mut port = SerialPortIO::new(
        &args.port,
        args.baud_rate,
        Duration::from_millis(args.timeout),
    )?;

    let user_opts = UserOptions::from(args);
    let mut state = State::Idle;

    loop {
        let packet = protocol::read_packet(&mut port)?;
        let (new_state, effects) = state.transition(packet, &mut port, &user_opts)?;

        for effect in effects {
            handle_effect(effect)?;
        }

        if let State::Finished(result) = new_state {
            return result;
        }

        state = new_state;
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    Ok(run(args)?)
}
