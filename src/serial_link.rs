//! Adapter that gives physical serial ports and pseudo-port files one I/O API.

use serialport::SerialPort;
use std::fs::File;
use std::io::{Read, Write};

pub(super) enum SerialLink {
    Physical(Box<dyn SerialPort>),
    Pseudo(File),
}

impl SerialLink {
    pub(super) fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            Self::Physical(port) => port
                .try_clone()
                .map(Self::Physical)
                .map_err(|e| std::io::Error::other(e.to_string())),
            Self::Pseudo(file) => file.try_clone().map(Self::Pseudo),
        }
    }
}

impl Read for SerialLink {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Physical(port) => port.read(buffer),
            Self::Pseudo(file) => file.read(buffer),
        }
    }
}

impl Write for SerialLink {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Physical(port) => port.write(buffer),
            Self::Pseudo(file) => file.write(buffer),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Physical(port) => port.flush(),
            Self::Pseudo(file) => file.flush(),
        }
    }
}
