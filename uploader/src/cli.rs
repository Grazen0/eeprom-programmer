use clap::Parser;
use derive_more::{Display, Error, From};
use std::{
    fs::File,
    io::{self, Read, Write},
    num::TryFromIntError,
    path::PathBuf,
};

use crate::client::{Client, ClientError};

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
}

pub type CliResult<T> = Result<T, CliError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    name: String,
    words: Vec<String>,
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;

        for arg in self.words.iter().skip(1) {
            write!(f, " {}", arg)?;
        }

        Ok(())
    }
}

fn parse_command(s: &str, cmd_idx: usize) -> CliResult<Command> {
    let words = shlex::split(s).ok_or(CliError::ParseError(cmd_idx + 1))?;

    let Some(name) = words.first().cloned() else {
        return Err(CliError::EmptyCommand(cmd_idx + 1));
    };

    Ok(Command { name, words })
}

pub fn parse_commands(strs: &[String]) -> CliResult<Vec<Command>> {
    let cmds: Vec<_> = strs
        .iter()
        .enumerate()
        .map(|(i, s)| parse_command(s, i))
        .collect::<Result<_, _>>()?;

    if cmds.is_empty() {
        return Err(CliError::NoCommandsProvided);
    }

    Ok(cmds)
}

const CHUNK_SIZE: usize = 48;
const ROM_CAPACITY: u64 = 0x8000;

#[derive(Debug, Clone, Parser)]
struct ReadArgs {
    #[arg(short, long, default_value_t = 0)]
    start: u16,

    #[arg(short, long, default_value_t = 0x8000)]
    len: usize,

    output: PathBuf,
}

#[derive(Debug, Clone, Parser)]
struct WriteArgs {
    #[arg(short, long)]
    no_verify: bool,

    file: PathBuf,
}

#[derive(Debug, Clone, Parser)]
struct LockArgs {}

#[derive(Debug, Clone, Parser)]
struct UnlockArgs {}

fn cmd_read(client: &mut Client, cmd: &[String]) -> Result<(), CliError> {
    let args = ReadArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;
    let mut out_file = File::create(&args.output)?;

    let len_digits = args.len.to_string().len();

    let mut read = 0;

    loop {
        const BAR_LEN: usize = 20;
        let progress = read as f32 / args.len as f32;
        let bar_filled = (progress * BAR_LEN as f32) as usize;

        print!(
            "\r  Reading... [{}{}] {:len_digits$}/{} bytes ({:6.2}%)",
            "#".repeat(bar_filled),
            ".".repeat(BAR_LEN - bar_filled),
            read,
            args.len,
            100.0 * progress
        );
        io::stdout().flush()?;

        if read >= args.len {
            break;
        }

        let to_read = CHUNK_SIZE.min(args.len - read);
        let chunk = client.read_data(args.start + u16::try_from(read)?, to_read.try_into()?)?;

        out_file.write_all(&chunk)?;
        read += chunk.len();
    }

    println!();
    println!("  {} bytes read to {:?}.", read, args.output);
    Ok(())
}

fn cmd_write(client: &mut Client, cmd: &[String]) -> Result<(), CliError> {
    let args = WriteArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;

    println!("  Opening {:?}...", args.file);
    let mut file = File::open(args.file)?;

    let file_len = file.metadata()?.len();
    if file_len > ROM_CAPACITY {
        return Err(CliError::FileTooLarge {
            got: file_len,
            max: ROM_CAPACITY,
        });
    }

    let file_len_digits = file_len.to_string().len();

    let mut written = 0;
    let mut buf = [0; CHUNK_SIZE];

    loop {
        const BAR_LEN: usize = 20;
        let progress = written as f32 / file_len as f32;
        let bar_filled = (progress * BAR_LEN as f32) as usize;

        print!(
            "\r  Writing... [{}{}] {:file_len_digits$}/{} bytes ({:6.2}%)",
            "#".repeat(bar_filled),
            ".".repeat(BAR_LEN - bar_filled),
            written,
            file_len,
            100.0 * progress
        );
        io::stdout().flush()?;

        let buf_len = file.read(&mut buf)?;
        if buf_len == 0 {
            break;
        }

        assert!(written + buf_len - 1 < ROM_CAPACITY as usize);

        client.write_data(&buf, written as u16)?;
        written += buf_len;
    }

    println!();
    println!("  {} bytes written.", written);
    Ok(())
}

fn cmd_lock(client: &mut Client, cmd: &[String]) -> Result<(), CliError> {
    let _args = LockArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;
    println!("  Locking EEPROM...");
    client.lock()?;
    println!("  EEPROM locked.");
    Ok(())
}

fn cmd_unlock(client: &mut Client, cmd: &[String]) -> Result<(), CliError> {
    let _args = UnlockArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;
    println!("  Unlocking EEPROM...");
    client.unlock()?;
    println!("  EEPROM unlocked.");
    Ok(())
}

fn cmd_erase(client: &mut Client, cmd: &[String]) -> Result<(), CliError> {
    let _args = UnlockArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;
    println!("  Erasing EEPROM...");
    client.erase()?;
    println!("  EEPROM erased.");
    Ok(())
}

pub fn exec_cmd(client: &mut Client, cmd: &Command) -> Result<(), CliError> {
    match cmd.name.as_str() {
        "read" => cmd_read(client, &cmd.words),
        "write" => cmd_write(client, &cmd.words),
        "lock" => cmd_lock(client, &cmd.words),
        "unlock" => cmd_unlock(client, &cmd.words),
        "erase" => cmd_erase(client, &cmd.words),
        name => Err(CliError::UnknownCommand(name.to_string())),
    }
}
