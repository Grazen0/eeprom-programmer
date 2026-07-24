mod cli;
mod client;
mod cobs;

use std::time::Duration;

use clap::Parser;

use crate::client::Client;

/// A program to interact with ATTIC EEPROM chips
#[derive(Debug, Clone, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "/dev/ttyUSB0")]
    port: String,

    #[arg(short, long, default_value_t = 115200)]
    baud_rate: u32,

    #[arg(short, long, default_value_t = 5.0)]
    timeout: f32,

    commands: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let cmds = cli::parse_commands(&args.commands)?;

    println!("Opening port at '{}'...", args.port);
    let port = serialport::new(args.port, args.baud_rate)
        .timeout(Duration::from_secs_f32(args.timeout))
        .open()?;
    println!("Port opened.");

    println!("Initializing client...");
    let mut client = Client::new(port)?;
    println!("Client initialized.");
    println!();

    for (i, cmd) in cmds.iter().enumerate() {
        print!("{}", "=".repeat(64));
        println!("\r({}) {} ", i + 1, cmd);
        println!();
        cli::exec_cmd(&mut client, cmd)?;
        println!();
    }

    Ok(())
}
