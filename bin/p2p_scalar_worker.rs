use clap::Parser;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use tungstenite::protocol::WebSocket;
use tungstenite::{accept, protocol::Message};
use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::p2p::scalar_worker::ExampleWorker;

#[derive(Parser, Debug)]
struct Args {
    /// The index of worker
    #[arg(long)]
    index: usize,

    /// The listening port
    #[arg(long)]
    port: u16,
}

fn main() {
    let args = Args::parse();

    let addr = format!("127.0.0.1:{}", args.port);
    println!("WebSocket server listening on: {}", addr);
    let listener = TcpListener::bind(addr).unwrap();

    let w = Arc::new(ExampleWorker::<Bls381K12Scalar> {
        index: args.index,
        peer_workers: Mutex::new(vec![]),
        stage_shares: Mutex::new(vec![]),
    });

    for stream in listener.incoming() {
        match stream {
            Err(e) => {
                eprintln!("accept failed: {}", e);
                continue;
            }
            Ok(stream) => {
                // Accept the WebSocket connection
                let websocket = accept(stream).unwrap();
                println!("New connection established.");
                let w = w.clone();
                thread::spawn(move || {
                    handle_connection(w, websocket);
                });
            }
        }
    }
}

fn handle_connection(w: Arc<ExampleWorker<Bls381K12Scalar>>, mut websocket: WebSocket<TcpStream>) {
    // Handle messages in a loop
    loop {
        let msg = websocket.read().unwrap();

        match msg {
            Message::Text(text) => {
                println!("Received: {}", text);

                // Assume the message format is "id:message"
                let parts: Vec<&str> = text.split(':').collect();
                if parts.len() == 2 {
                    let id = parts[0];
                    let response = format!("{}: Echo: {}", id, parts[1]);
                    websocket.send(Message::Text(response)).unwrap();
                } else {
                    println!("Invalid message format.");
                }
            }
            Message::Close(_) => {
                println!("Connection closed.");
                break;
            }
            _ => (),
        }
    }
}
