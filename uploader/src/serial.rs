use serialport::SerialPort;

pub fn write_u16(port: &mut Box<dyn SerialPort>, value: u16) -> serialport::Result<()> {
    port.write_all(&value.to_ne_bytes())?;
    Ok(())
}

pub fn read_u8(port: &mut Box<dyn SerialPort>) -> serialport::Result<u8> {
    let mut buf = [0];

    while port.bytes_to_read()? == 0 {}

    port.read_exact(&mut buf)?;
    Ok(buf[0])
}

pub fn read_n_bytes(port: &mut Box<dyn SerialPort>, n: usize) -> serialport::Result<Vec<u8>> {
    while port.bytes_to_read()? < n.try_into().unwrap() {}

    let mut buf = vec![0; n];
    port.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn read_u16(port: &mut Box<dyn SerialPort>) -> serialport::Result<u16> {
    let lo = read_u8(port)?;
    let hi = read_u8(port)?;
    Ok(u16::from_ne_bytes([lo, hi]))
}
