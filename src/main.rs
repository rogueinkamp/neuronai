mod ncp;

use std::{
    env,
    io::{ErrorKind, Read},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use ncp::{connect, listen, Message, SignalType};

const DISCOVERY_PORT: u16 = 5002;
const NEURON_PORT_START: u16 = 5003;
const DISCOVERY_INTERVAL_MS: u64 = 1000;
const ANNOUNCE_INTERVAL_MS: u64 = 2000;

#[derive(Debug)]
struct Neuron {
    id: u16,
    address: SocketAddr,
    peers: Arc<Mutex<Vec<SocketAddr>>>,
    discovery_send_socket: std::net::UdpSocket, // Add a socket for sending discovery
}

impl Neuron {
    fn new(id: u16) -> Self {
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), NEURON_PORT_START + id);
        let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0); // Bind to any available port
        let discovery_send_socket =
            std::net::UdpSocket::bind(bind_address).expect("Failed to bind UDP socket for sending");
        discovery_send_socket.set_broadcast(true).expect("Failed to set broadcast");

        Neuron {
            id,
            address,
            peers: Arc::new(Mutex::new(Vec::new())),
            discovery_send_socket,
        }
    }

    fn announce_presence(self: Arc<Self>) -> Result<(), std::io::Error> {
        let discovery_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), DISCOVERY_PORT);
        let announcement = self.address.to_string().as_bytes().to_vec();
        self.discovery_send_socket.send_to(&announcement, discovery_address)?;
        println!("Neuron {} ({}) announced its presence. | peers={:?}", self.id, self.address, self.peers);
        Ok(())
    }

    fn listen_for_announcements(self: Arc<Self>) {
        let discovery_listen_address =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DISCOVERY_PORT + self.id); // Use a unique port

        let discovery_socket =
            std::net::UdpSocket::bind(discovery_listen_address)
                .expect("Failed to bind to discovery port");

        let peers = Arc::clone(&self.peers);
        let self_address = self.address;
        let self_id = self.id;

        thread::spawn(move || {
            let mut buffer = [0; 1024];
            loop {
                match discovery_socket.recv_from(&mut buffer) {
                    Ok((size, src_address)) => {
                        if src_address != self_address {
                            if let Ok(peer_address_str) = String::from_utf8(buffer[..size].to_vec()) {
                                if let Ok(peer_address) = peer_address_str.parse::<SocketAddr>() {
                                    let mut peers_guard = peers.lock().unwrap();
                                    if !peers_guard.contains(&peer_address) {
                                        println!(
                                            "Neuron {} ({}) discovered peer: {}",
                                            self_id, self_address, peer_address
                                        );
                                        peers_guard.push(peer_address);

                                    }
                                } else {
                                    eprintln!(
                                        "Neuron {} ({}) received invalid address: {}",
                                        self_id, self_address, peer_address_str
                                    );
                                }
                            } else {
                                eprintln!(
                                    "Neuron {} ({}) received non-UTF8 data from: {}",
                                    self_id, self_address, src_address
                                );
                            }
                        }
                    }
                    Err(e) => {
                        if e.kind() != ErrorKind::WouldBlock {
                            eprintln!(
                                "Neuron {} ({}) error receiving discovery: {}",
                                self_id, self_address, e
                            );
                        }
                    }
                }

                thread::sleep(Duration::from_millis(DISCOVERY_INTERVAL_MS));
            }
        });
    }

    fn connect_to_peers(self: Arc<Self>) {
        let peers = Arc::clone(&self.peers);
        let self_address = self.address;
        let self_id = self.id;

        thread::spawn(move || {
            loop {
                let peers_guard = peers.lock().unwrap();
                for peer_address in &*peers_guard {
                    if *peer_address < self_address {
                        println!(
                            "Neuron {} ({}) attempting to connect to peer: {}",
                            self_id, self_address, peer_address
                        );
                        match connect(&peer_address.to_string()) {
                            Ok(mut stream) => {
                                println!(
                                    "Neuron {} ({}) successfully connected to peer: {}",
                                    self_id, self_address, peer_address
                                );
                                // You can now use 'stream' to send and receive NCP messages
                                // For example, send an initial handshake or discovery confirmation
                                if let Err(e) = Message::new(self_id, SignalType::Data, self_id as f32)
                                    .send(&mut stream)
                                {
                                    eprintln!(
                                        "Neuron {} ({}) error sending initial message to {}: {}",
                                        self_id, self_address, peer_address, e
                                    );
                                }
                                // Handle communication with this peer in a separate thread or loop
                                Self::handle_peer_communication(Arc::clone(&self), stream, *peer_address);
                            }
                            Err(e) => {
                                eprintln!(
                                    "Neuron {} ({}) error connecting to {}: {}",
                                    self_id, self_address, peer_address, e

                                );
                            }
                        }
                    }
                }
                drop(peers_guard); // Release the lock
                thread::sleep(Duration::from_secs(5)); // Attempt connections periodically
            }
        });

    }

    fn handle_peer_communication(self: Arc<Self>, mut stream: TcpStream, peer_address: SocketAddr) {
        let self_id = self.id;
        let self_address = self.address;
        thread::spawn(move || {
            println!(
                "Neuron {} ({}) handling communication with peer: {}",
                self_id, self_address, peer_address
            );
            loop {
                match stream.read(&mut [0; 128]) {
                    Ok(0) => {
                        println!(
                            "Neuron {} ({}) peer {} disconnected.",

                            self_id, self_address, peer_address
                        );
                        break;
                    }
                    Ok(size) => {
                        println!(
                            "Neuron {} ({}) received {} bytes from {}",
                            self_id, self_address, size, peer_address
                        );
                        // Here you would use ncp::Message::receive(&mut stream)
                        match Message::receive(&mut stream) {
                            Ok(Some(message)) => {
                                println!(
                                    "Neuron {} ({}) received NCP message from {}: {:?}",

                                    self_id, self_address, peer_address, message
                                );
                                // Process the received message (e.g., for election)
                            }
                            Ok(None) => {
                                println!(
                                    "Neuron {} ({}) peer {} likely disconnected gracefully (NCP).",
                                    self_id, self_address, peer_address
                                );
                                break;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Neuron {} ({}) error receiving NCP message from {}: {}",
                                    self_id, self_address, peer_address, e
                                );
                                break;

                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Neuron {} ({}) error reading from {}: {}",
                            self_id, self_address, peer_address, e
                        );
                        break;
                    }
                }
            }
        });
    }

    fn listen_for_connections(self: Arc<Self>) {
        let listener = listen(&self.address.to_string()).expect("Failed to start listener");
        let self_id = self.id;
        let self_address = self.address;
        let peers = Arc::clone(&self.peers);

        thread::spawn(move || {
            println!("Neuron {} ({}) listening for incoming connections.", self_id, self_address);
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let peer_address = stream.peer_addr().unwrap();
                        println!(
                            "Neuron {} ({}) accepted connection from: {}",
                            self_id, self_address, peer_address
                        );
                        let mut peers_guard = peers.lock().unwrap();
                        if !peers_guard.contains(&peer_address) {
                            peers_guard.push(peer_address);
                        }
                        drop(peers_guard);
                        Self::handle_peer_communication(Arc::clone(&self), stream, peer_address);
                    }
                    Err(e) => {
                        eprintln!(
                            "Neuron {} ({}) error accepting connection: {}",
                            self_id, self_address, e
                        );
                    }
                }
            }
            println!("Neuron {} ({}) listener stopped.", self_id, self_address);
        });
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <number_of_neurons>", args[0]);
        return;
    }
    let num_neurons_str = &args[1];
    let num_neurons = match num_neurons_str.parse::<u16>() {
        Ok(n) if n > 0 && n <= 65535 - NEURON_PORT_START => n,
        _ => {
            eprintln!("Invalid number of neurons. Must be between 1 and {}", 65535 - NEURON_PORT_START);
            return;
        }
    };

    let mut neurons = Vec::new();
    let mut neuron_handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
    for i in 0..num_neurons {
        let neuron = Arc::new(Neuron::new(i));
        Arc::clone(&neuron).listen_for_announcements(); // Clone before calling
        Arc::clone(&neuron).listen_for_connections(); // Clone before calling
        Arc::clone(&neuron).connect_to_peers();     // Clone before calling
        neurons.push(Arc::clone(&neuron));

        // Announce presence periodically
        let neuron_clone = Arc::clone(&neuron);
        let handle = thread::spawn(move || {
            loop {
                if let Err(e) = Arc::clone(&neuron_clone).announce_presence() {
                    eprintln!(
                        "Neuron {} ({}) error announcing: {} | peers {:?}",
                        neuron_clone.id, neuron_clone.address, e, neuron_clone.peers
                    );
                }
                thread::sleep(Duration::from_millis(ANNOUNCE_INTERVAL_MS));
            }
        });
        neuron_handles.push(handle);
    }

    println!("Created {} neurons.", num_neurons);
    // Keep the main thread alive to allow the neuron threads to run
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
