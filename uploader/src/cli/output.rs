use std::io::{self, StdoutLock, Write};

#[derive(Debug)]
pub struct ProgressMeter {
    status: &'static str,
    total: u64,
    total_digits: usize,
    stdout: StdoutLock<'static>,
}

impl ProgressMeter {
    pub fn new(status: &'static str, total: u64) -> Self {
        let stdout = io::stdout().lock();

        Self {
            status,
            total,
            total_digits: total.to_string().len(),
            stdout,
        }
    }

    pub fn update(&mut self, progress: u64) -> io::Result<()> {
        const BAR_LEN: usize = 20;

        let percent = progress as f32 / self.total as f32;
        let bar_filled = (percent * BAR_LEN as f32) as usize;

        write!(
            self.stdout,
            "\r  {:12} [{}{}] {:6$}/{} ({:3.0}%)",
            self.status,
            "#".repeat(bar_filled),
            ".".repeat(BAR_LEN - bar_filled),
            progress,
            self.total,
            100.0 * percent,
            self.total_digits,
        )?;
        self.stdout.flush()?;
        Ok(())
    }
}

impl Drop for ProgressMeter {
    fn drop(&mut self) {
        let _ = writeln!(self.stdout);
    }
}
