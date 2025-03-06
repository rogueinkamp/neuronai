// Neuron Communication Protocol
// Lightweigh socket programming to facilitate communication between Neurons

use std::net::{TcpStream, TcpListener};
use std::io::{Read, Write, Result};

/// Represents a message in the Neuron Communication Protocol (NCP)
pub struct Message {
    pub sender_id: u16,    // 2 bytes: Neuron ID (0-65535)
    pub signal_type: u8,   // 1 byte: Type of signal (0 = data, expandable later)
    pub value: f32,        // 4 bytes: Scalar value for now
}


impl Message {
    /// Create a new NCP message
    pub fn new(sender_id: u16, signal_type: u8, value: f32) -> Self {
        Message {
            sender_id,
            signal_type,
            value,
        }
    }

    /// Serialize and send the message over a TCP stream
    pub fn send(&self, stream: &mut TcpStream) -> Result<()> {
        let bytes = [
            self.sender_id.to_be_bytes().as_slice(), // Big-endian for network
            &[self.signal_type],
            self.value.to_be_bytes().as_slice(),

        ].concat();

        stream.write_all(&bytes)?;
        stream.flush()?;

        Ok(())
    }

    /// Receive a message from a TCP stream
    pub fn receive(stream: &mut TcpStream) -> Result<Self> {
        let mut buffer = [0u8; 7]; // 2 + 1 + 4 = 7 bytes
        stream.read_exact(&mut buffer)?;
        let sender_id = u16::from_be_bytes([buffer[0], buffer[1]]);
        let signal_type = buffer[2];
        let value = f32::from_be_bytes([buffer[3], buffer[4], buffer[5], buffer[6]]);

        Ok(Message {
            sender_id,
            signal_type,
            value,
        })
    }
}

/// Listen for incoming NCP messages on a given address
pub fn listen(address: &str) -> Result<TcpListener> {
    let listener = TcpListener::bind(address)?;
    Ok(listener)
}

/// Connect to a remote NCP endpoint
pub fn connect(address: &str) -> Result<TcpStream> {
    let stream = TcpStream::connect(address)?;
    Ok(stream)
}
