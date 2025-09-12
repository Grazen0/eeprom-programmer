use std::time::Duration;

use serialport::SerialPort;

pub trait SerialIO {
    fn read_u8(&mut self) -> anyhow::Result<u8>;
    fn read_u16(&mut self) -> anyhow::Result<u16>;
    fn read_n(&mut self, n: usize) -> anyhow::Result<Vec<u8>>;

    fn write_u8(&mut self, value: u8) -> anyhow::Result<()>;
    fn write_u16(&mut self, value: u16) -> anyhow::Result<()>;
    fn write_n(&mut self, data: &[u8]) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct SerialPortIO {
    port: Box<dyn SerialPort>,
}

impl SerialPortIO {
    pub fn new(path: &str, baud_rate: u32, timeout: Duration) -> anyhow::Result<Self> {
        let port = serialport::new(path, baud_rate).timeout(timeout).open()?;

        Ok(SerialPortIO { port })
    }
}

impl SerialIO for SerialPortIO {
    fn read_u8(&mut self) -> anyhow::Result<u8> {
        let mut buf = [0];

        while self.port.bytes_to_read()? == 0 {}

        self.port.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> anyhow::Result<u16> {
        while self.port.bytes_to_read()? < 2 {}

        let mut buf = [0; 2];
        self.port.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_n(&mut self, n: usize) -> anyhow::Result<Vec<u8>> {
        while self.port.bytes_to_read()? < n.try_into().unwrap() {}

        let mut buf = vec![0; n];
        self.port.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn write_u8(&mut self, value: u8) -> anyhow::Result<()> {
        self.port.write_all(&[value])?;
        Ok(())
    }

    fn write_u16(&mut self, value: u16) -> anyhow::Result<()> {
        self.port.write_all(&value.to_be_bytes())?;
        Ok(())
    }

    fn write_n(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.port.write_all(data)?;
        Ok(())
    }
}
