use clap::Parser;
use std::{
    fs::File,
    io::{BufReader, Read, Seek, Write},
    path::PathBuf,
    slice,
};

use crate::{
    cli::output::ProgressMeter,
    client::{BlockReader, Client},
};

mod error;
mod output;
mod parser;

const CHUNK_SIZE: u64 = 248;
const ROM_CAPACITY: u64 = 0x8000;

pub use error::*;
pub use parser::*;

#[derive(Debug, Clone, Parser)]
struct ReadArgs {
    #[arg(short, long, default_value_t = 0)]
    start: u16,

    #[arg(short, long, default_value_t = 0x8000)]
    len: u64,

    output: PathBuf,
}

#[derive(Debug, Clone, Parser)]
struct WriteArgs {
    #[arg(long)]
    no_verify: bool,

    file: PathBuf,
}

#[derive(Debug, Clone, Parser)]
struct VerifyArgs {
    #[arg(long)]
    fix: bool,

    file: PathBuf,
}

#[derive(Debug, Clone, Parser)]
struct LockArgs {}

#[derive(Debug, Clone, Parser)]
struct UnlockArgs {}

fn cmd_read(client: &mut Client, cmd: &[String]) -> CliResult<()> {
    let args = ReadArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;
    let mut out_file = File::create(&args.output)?;
    let mut read = 0_u64;

    {
        let mut meter = ProgressMeter::new("Reading", args.len);

        loop {
            meter.update(read)?;

            if read >= args.len {
                break;
            }

            let to_read = CHUNK_SIZE.min(args.len - read);
            let chunk = client.read_data(args.start + u16::try_from(read)?, to_read.try_into()?)?;

            out_file.write_all(&chunk)?;
            read += chunk.len() as u64;
        }
    }

    println!("  {} bytes read to {:?}.", read, args.output);
    Ok(())
}

fn cmd_write(client: &mut Client, cmd: &[String]) -> CliResult<()> {
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

    let mut written = 0;
    let mut buf = [0; CHUNK_SIZE as usize];

    {
        let mut meter = ProgressMeter::new("Writing", file_len);

        loop {
            meter.update(written as u64)?;

            let buf_len = file.read(&mut buf)?;
            if buf_len == 0 {
                break;
            }

            assert!(written + buf_len - 1 < ROM_CAPACITY as usize);

            client.write_data(&buf, written as u16)?;
            written += buf_len;
        }
    }

    println!("  {} bytes written.", written);

    if !args.no_verify {
        verify(client, &mut file, true)?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cell {
    addr: u16,
    val: u8,
}

fn verify(client: &mut Client, file: &mut File, fix: bool) -> CliResult<()> {
    let file_len = file.metadata()?.len();
    if file_len > ROM_CAPACITY {
        return Err(CliError::FileTooLarge {
            got: file_len,
            max: ROM_CAPACITY,
        });
    }

    let mut file_reader = BufReader::new(&mut *file);
    file_reader.rewind()?;

    let mut eeprom_reader = BlockReader::new(client, CHUNK_SIZE as u8);
    let mut mismatches = vec![];

    {
        let mut meter = ProgressMeter::new("Verifying", file_len);
        let mut addr = 0_u16;

        loop {
            meter.update(addr.into())?;
            if !mismatches.is_empty() {
                print!(" - mismatches: {}", mismatches.len());
            }

            if addr >= file_len as u16 {
                break;
            }

            let mut exp = 0;
            file_reader.read_exact(slice::from_mut(&mut exp))?;
            let got = eeprom_reader.read(addr)?;

            if exp != got {
                mismatches.push(Cell { addr, val: exp });
            }

            addr += 1;
        }
    }

    if mismatches.is_empty() {
        println!("  No mismatches found.");
    } else if fix {
        {
            let mut meter = ProgressMeter::new("Fixing", mismatches.len() as u64);
            let mut i = 0;

            loop {
                meter.update(i as u64)?;

                if i >= mismatches.len() {
                    break;
                }

                let Cell { addr, val } = mismatches[i];

                client.write_data(&[val], addr)?;
                i += 1;
            }
        }

        verify(client, file, fix)?;
    } else {
        println!("{} mismatches found.", mismatches.len());
    }

    Ok(())
}

fn cmd_verify(client: &mut Client, cmd: &[String]) -> CliResult<()> {
    let args = VerifyArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;

    println!("  Opening {:?}...", args.file);
    let mut file = File::open(args.file)?;

    verify(client, &mut file, args.fix)?;
    Ok(())
}

fn cmd_lock(client: &mut Client, cmd: &[String]) -> CliResult<()> {
    let _args = LockArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;
    println!("  Locking EEPROM...");
    client.lock()?;
    println!("  EEPROM locked.");
    Ok(())
}

fn cmd_unlock(client: &mut Client, cmd: &[String]) -> CliResult<()> {
    let _args = UnlockArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;
    println!("  Unlocking EEPROM...");
    client.unlock()?;
    println!("  EEPROM unlocked.");
    Ok(())
}

fn cmd_erase(client: &mut Client, cmd: &[String]) -> CliResult<()> {
    let _args = UnlockArgs::try_parse_from(cmd).map_err(|e| CliError::Clap(e.to_string()))?;
    println!("  Erasing EEPROM...");
    client.erase()?;
    println!("  EEPROM erased.");
    Ok(())
}

pub fn exec_cmd(client: &mut Client, cmd: &Command) -> CliResult<()> {
    match cmd.name() {
        "read" => cmd_read(client, cmd.words()),
        "write" => cmd_write(client, cmd.words()),
        "verify" => cmd_verify(client, cmd.words()),
        "lock" => cmd_lock(client, cmd.words()),
        "unlock" => cmd_unlock(client, cmd.words()),
        "erase" => cmd_erase(client, cmd.words()),
        name => Err(CliError::UnknownCommand(name.to_string())),
    }
}
