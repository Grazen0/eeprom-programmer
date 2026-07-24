use crate::cli::error::{CliError, CliResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    name: String,
    words: Vec<String>,
}

impl Command {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn words(&self) -> &[String] {
        &self.words
    }
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
