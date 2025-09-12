use std::{fs::File, io::Write, path::PathBuf};

use derive_more::{Display, Error, From};

use crate::{
    protocol::{self, Packet, ProtocolError},
    serial::SerialIO,
};

#[derive(Debug, From, Display, Error)]
pub enum Error {
    #[display("I/O error: {_0}")]
    IO(#[from] std::io::Error),

    Protocol(#[from] ProtocolError),

    #[display("Board sent an invalid opcode: {_0}")]
    InvalidOpcode(#[error(not(source))] u8),

    #[display("Received an unexpected packet (state: {state_kind:?}, packet: {packet})")]
    UnexpectedPacket {
        state_kind: StateKind,
        packet: Packet,
    },

    #[display("Checksum mismatch (expected = 0x{expected:04X}, computed = 0x{computed:04X})")]
    ChecksumMismatch {
        expected: u16,
        computed: u16,
    },

    #[display("Memory region bounds must be valid")]
    InvalidRegionBounds,

    #[display("Unknown: {_0}")]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub enum UserCommand {
    Read {
        out_filename: PathBuf,
        start: u16,
        end: u16,
    },
    Write {
        in_filename: PathBuf,
        verify: bool,
    },
    Verify {
        in_filename: PathBuf,
        fix: bool,
    },
}

#[derive(Debug, Clone)]
pub struct UserOptions {
    pub command: UserCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateKind {
    Idle,
    Reading,
    Writing,
    Verifying,
    Fixing,
    Finished,
}

#[derive(Debug, Clone)]
pub enum Effect {
    Print(String),
    PrintLn(String),
    Progress {
        done: usize,
        total: usize,
    },
    VerifyProgress {
        done: usize,
        total: usize,
        mismatches: usize,
    },
    ProgressEnd,
}

#[derive(Debug, Clone)]
pub struct ByteMismatch {
    address: u16,
    expected: u8,
}

#[derive(Debug)]
pub enum State {
    Idle,
    Reading {
        progress: usize,
        total: usize,
        out_file: File,
        out_path: PathBuf,
    },
    Writing {
        current_byte: usize,
        data: Vec<u8>,
        verify: bool,
    },
    Verifying {
        current_byte: usize,
        data: Vec<u8>,
        mismatches: Vec<ByteMismatch>,
        fix: bool,
    },
    Fixing {
        mismatches: Vec<ByteMismatch>,
        current: usize,
    },
    Finished(Result<(), Error>),
}

impl State {
    pub fn kind(&self) -> StateKind {
        match self {
            Self::Idle => StateKind::Idle,
            Self::Reading { .. } => StateKind::Reading,
            Self::Writing { .. } => StateKind::Writing,
            Self::Verifying { .. } => StateKind::Verifying,
            Self::Fixing { .. } => StateKind::Fixing,
            Self::Finished(_) => StateKind::Finished,
        }
    }

    pub fn transition(
        self,
        packet: Packet,
        port: &mut impl SerialIO,
        opts: &UserOptions,
    ) -> Result<(State, Vec<Effect>), Error> {
        let mut effects = vec![];

        let next_state = match (self, packet) {
            (_, Packet::Ready) => match opts.command {
                UserCommand::Read {
                    ref out_filename,
                    start,
                    end,
                } => {
                    if end < start {
                        return Err(Error::InvalidRegionBounds);
                    }

                    let out_file = File::create(out_filename)?;

                    effects.push(Effect::PrintLn("Initiating EEPROM read...".to_owned()));

                    port.write_u8(0x00)?;
                    port.write_u16(start)?;
                    port.write_u16(end)?;

                    State::Reading {
                        progress: 0,
                        total: (end - start).into(),
                        out_file,
                        out_path: out_filename.clone(),
                    }
                }
                UserCommand::Write {
                    ref in_filename,
                    verify,
                } => {
                    effects.push(Effect::PrintLn("Initiating EEPROM write...".to_owned()));

                    let data = std::fs::read(in_filename)?;

                    port.write_u8(0x01)?;
                    port.write_u8(verify.into())?;

                    State::Writing {
                        current_byte: 0,
                        data,
                        verify,
                    }
                }
                UserCommand::Verify {
                    ref in_filename,
                    fix,
                } => {
                    let data = std::fs::read(in_filename)?;

                    effects.push(Effect::PrintLn(
                        "Initiating EEPROM verification...".to_owned(),
                    ));

                    port.write_u8(0x02)?;
                    port.write_u8(fix.into())?;

                    State::Verifying {
                        current_byte: 0,
                        data,
                        mismatches: vec![],
                        fix,
                    }
                }
            },
            (state, Packet::Print(s)) => {
                effects.push(Effect::Print(s));
                state
            }
            (_, Packet::InvalidChecksum { expected, computed }) => {
                State::Finished(Err(Error::ChecksumMismatch { expected, computed }))
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
                let computed_checksum = protocol::calculate_checksum(&chunk_data);

                if checksum != computed_checksum {
                    State::Finished(Err(Error::ChecksumMismatch {
                        expected: checksum,
                        computed: computed_checksum,
                    }))
                } else {
                    let new_progress = progress + chunk_data.len();
                    out_file.write_all(&chunk_data)?;

                    port.write_u8(0xFF)?;

                    effects.push(Effect::Progress {
                        done: new_progress,
                        total,
                    });

                    State::Reading {
                        progress: new_progress,
                        total,
                        out_file,
                        out_path,
                    }
                }
            }
            (State::Reading { out_path, .. }, Packet::ReadEnd) => {
                effects.push(Effect::ProgressEnd);
                effects.push(Effect::PrintLn(format!(
                    "Memory contents successfully dumped to {:?}",
                    out_path
                )));

                State::Finished(Ok(()))
            }

            (
                State::Writing {
                    current_byte,
                    data,
                    verify,
                },
                Packet::ChunkRequest,
            ) if current_byte >= data.len() => {
                effects.push(Effect::ProgressEnd);
                effects.push(Effect::PrintLn(format!(
                    "{} bytes successfully written to EEPROM.",
                    data.len()
                )));

                port.write_u8(0x00)?;

                if verify {
                    effects.push(Effect::PrintLn("Verifying...".to_owned()));

                    State::Verifying {
                        data,
                        current_byte: 0,
                        mismatches: vec![],
                        fix: true,
                    }
                } else {
                    State::Finished(Ok(()))
                }
            }
            (
                State::Writing {
                    mut current_byte,
                    data,
                    verify,
                },
                Packet::ChunkRequest,
            ) => {
                protocol::send_data_chunk(port, &data, &mut current_byte)?;

                effects.push(Effect::Progress {
                    done: current_byte,
                    total: data.len(),
                });

                State::Writing {
                    current_byte,
                    data,
                    verify,
                }
            }

            (
                State::Verifying {
                    data,
                    current_byte,
                    mut mismatches,
                    fix,
                },
                Packet::ByteMismatch {
                    address, expected, ..
                },
            ) => {
                effects.push(Effect::VerifyProgress {
                    done: current_byte,
                    total: data.len(),
                    mismatches: mismatches.len(),
                });

                mismatches.push(ByteMismatch { address, expected });

                State::Verifying {
                    data,
                    current_byte,
                    mismatches,
                    fix,
                }
            }
            (
                State::Verifying {
                    data,
                    current_byte,
                    mismatches,
                    fix,
                },
                Packet::ChunkRequest,
            ) if current_byte >= data.len() => {
                port.write_u8(0x00)?;

                effects.push(Effect::ProgressEnd);

                if mismatches.is_empty() {
                    effects.push(Effect::PrintLn("No mismatches found.".to_owned()));
                    State::Finished(Ok(()))
                } else {
                    effects.push(Effect::PrintLn(format!(
                        "{} mismatches found.",
                        mismatches.len()
                    )));

                    if fix {
                        State::Fixing {
                            mismatches,
                            current: 0,
                        }
                    } else {
                        State::Finished(Ok(()))
                    }
                }
            }
            (
                State::Verifying {
                    mut current_byte,
                    data,
                    mismatches,
                    fix,
                },
                Packet::ChunkRequest,
            ) => {
                protocol::send_data_chunk(&mut *port, &data, &mut current_byte)?;

                effects.push(Effect::VerifyProgress {
                    done: current_byte,
                    total: data.len(),
                    mismatches: mismatches.len(),
                });

                State::Verifying {
                    current_byte,
                    data,
                    mismatches,
                    fix,
                }
            }

            (
                State::Fixing {
                    mismatches,
                    current,
                },
                Packet::ByteRequest,
            ) if current >= mismatches.len() => {
                port.write_u16(0xFFFF)?;

                effects.push(Effect::ProgressEnd);
                effects.push(Effect::PrintLn("Mismatches fixed successfully.".to_owned()));

                State::Finished(Ok(()))
            }
            (
                State::Fixing {
                    mismatches,
                    mut current,
                },
                Packet::ByteRequest,
            ) => {
                let mismatch = &mismatches[current];
                current += 1;

                port.write_u16(mismatch.address)?;
                port.write_u8(mismatch.expected)?;

                effects.push(Effect::Progress {
                    total: mismatches.len(),
                    done: current,
                });
                State::Fixing {
                    mismatches,
                    current,
                }
            }

            (state, packet) => State::Finished(Err(Error::UnexpectedPacket {
                state_kind: state.kind(),
                packet,
            })),
        };

        Ok((next_state, effects))
    }
}
