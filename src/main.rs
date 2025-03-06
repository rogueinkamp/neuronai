mod ncp;
use std::io;
use std::thread;


fn main() -> io::Result<()> {
    let listener_addr = "127.0.0.1:8080";
    let listener = ncp::listen(listener_addr)?;
    println!("Listening perpetually on {}", listener_addr);

    // Main loop: accept connections forever
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                // Spawn a thread per connection to handle multiple clients
                thread::spawn(move || {
                    match ncp::Message::receive(&mut stream) {

                        Ok(msg) => {
                            println!(
                                "Received: sender_id={}, signal_type={}, value={}",

                                msg.sender_id, msg.signal_type, msg.value
                            );
                            // Echo back the message (optional, for testing)
                            if let Err(e) = msg.send(&mut stream) {
                                eprintln!("Send error: {}", e);
                            }
                        }

                        Err(e) => eprintln!("Receive error: {}", e),
                    }
                });
            }
            Err(e) => eprintln!("Connection error: {}", e),
        }
    }
    // Unreachable, but keeps the signature happy
    Ok(())
}
