use super::common::{SenderMsg, ReceiverMsg};

use message_io::events::{EventQueue};
use message_io::network::{NetworkManager, NetEvent, Endpoint};

use std::collections::{HashMap};
use std::fs::{File};
use std::io::{Write};

pub struct Transfer {
    file: File,
    name: String,
    current_size: usize,
    expected_size: usize,
}

enum Event {
    Network(NetEvent<SenderMsg>),
}

pub fn run() {
    let mut event_queue = EventQueue::new();

    let network_sender = event_queue.sender().clone();
    let mut network =
        NetworkManager::new(move |net_event| network_sender.send(Event::Network(net_event)));

    let listen_addr = "127.0.0.1:3005";
    match network.listen_tcp(listen_addr) {
        Ok(_) => println!("Receiver running by TCP at {}", listen_addr),
        Err(_) => return println!("Can not listening by TCP at {}", listen_addr),
    }

    let mut transfers: HashMap<Endpoint, Transfer> = HashMap::new();

    loop {
        match event_queue.receive() {
            Event::Network(net_event) => match net_event {
                NetEvent::Message(endpoint, message) => match message {
                    SenderMsg::FileRequest(name, size) => {
                        let able = match File::create(format!("{}.recv", name)) {
                            Ok(file) => {
                                println!("Accept file: '{}' with {} bytes", name, size);
                                let transfer =
                                    Transfer { file, name, current_size: 0, expected_size: size };
                                transfers.insert(endpoint, transfer);
                                true
                            }
                            Err(_) => {
                                println!("Can not open the file to write");
                                false
                            }
                        };

                        network.send(endpoint, ReceiverMsg::CanReceive(able)).unwrap();
                    }
                    SenderMsg::Chunk(data) => {
                        let transfer = transfers.get_mut(&endpoint).unwrap();
                        transfer.file.write(&data).unwrap();
                        transfer.current_size += data.len();

                        let current = transfer.current_size as f32;
                        let expected = transfer.expected_size as f32;
                        let percentage = ((current / expected) * 100.0) as usize;
                        print!("\rReceiving '{}': {}%", transfer.name, percentage);

                        if transfer.expected_size == transfer.current_size {
                            println!("\nFile '{}' received!", transfer.name);
                            transfers.remove(&endpoint).unwrap();
                        }
                    }
                },
                NetEvent::AddedEndpoint(_) => {}
                NetEvent::RemovedEndpoint(endpoint) => {
                    // Unexpected sender disconnection. Cleaninig.
                    if transfers.contains_key(&endpoint) {
                        println!("\nUnexpected Sender disconnected");
                        transfers.remove(&endpoint);
                    }
                }
            },
        }
    }
}
