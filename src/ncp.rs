// Neuron Communication Protocol
// Lightweigh socket programming to facilitate communication between Neurons


use std::net::{TcpStream, TcpListener};
use std::io::{Read, Write, Result};

/// Represents the different types of signals in the Neuron Communication Protocol (NCP)
#[derive(Debug)]
pub enum SignalType {
    Data = 0,
    ElectionRequest = 1,
    Nomination = 2,
    Vote = 3,
    Victory = 4,
}

impl SignalType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(SignalType::Data),
            1 => Some(SignalType::ElectionRequest),
            2 => Some(SignalType::Nomination),
            3 => Some(SignalType::Vote),
            4 => Some(SignalType::Victory),
            _ => None,
        }

    }

    pub fn to_u8(&self) -> u8 {
        match self {
            SignalType::Data => 0,
            SignalType::ElectionRequest => 1,
            SignalType::Nomination => 2,
            SignalType::Vote => 3,
            SignalType::Victory => 4,

        }
    }
}

/// Represents a message in the Neuron Communication Protocol (NCP)
#[derive(Debug)]
pub struct Message {
    pub sender_id: u16,     // 2 bytes: Neuron ID (0-65535)
    pub signal_type: SignalType, // 1 byte: Type of signal
    pub value: f32,         // 4 bytes: Scalar value (can be repurposed for election data)
}


impl Message {
    /// Create a new NCP message
    pub fn new(sender_id: u16, signal_type: SignalType, value: f32) -> Self {
        Message {
            sender_id,
            signal_type,
            value,
        }

    }

    /// Create a new NCP message with a SignalType from a u8
    pub fn new_with_u8_signal(sender_id: u16, signal_type: u8, value: f32) -> Option<Self> {

        SignalType::from_u8(signal_type).map(|st| Message {
            sender_id,
            signal_type: st,
            value,
        })
    }

    /// Serialize and send the message over a TCP stream
    pub fn send(&self, stream: &mut TcpStream) -> Result<()> {
        let bytes = [
            self.sender_id.to_be_bytes().as_slice(), // Big-endian for network
            &[self.signal_type.to_u8()],
            self.value.to_be_bytes().as_slice(),
        ].concat();

        stream.write_all(&bytes)?;
        stream.flush()?;

        Ok(())
    }


    /// Receive a message from a TCP stream
    pub fn receive(stream: &mut TcpStream) -> Result<Option<Self>> {

        let mut buffer = [0u8; 7]; // 2 + 1 + 4 = 7 bytes
        match stream.peek(&mut buffer) {
            Ok(0) => return Ok(None), // Connection closed
            Ok(_) => {
                stream.read_exact(&mut buffer)?;
                let sender_id = u16::from_be_bytes([buffer[0], buffer[1]]);
                let signal_type_u8 = buffer[2];
                let value = f32::from_be_bytes([buffer[3], buffer[4], buffer[5], buffer[6]]);

                if let Some(signal_type) = SignalType::from_u8(signal_type_u8) {
                    Ok(Some(Message {
                        sender_id,
                        signal_type,
                        value,
                    }))
                } else {
                    // Handle invalid signal type (optional: could return an error instead)
                    eprintln!("Warning: Received message with invalid signal type: {}", signal_type_u8);
                    Ok(None)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None), // No data available yet
            Err(e) => Err(e),
        }
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
