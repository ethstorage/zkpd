use clap::Parser;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use tungstenite::protocol::WebSocket;
use tungstenite::{accept, protocol::Message};
use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::mode::scalar::{Base, Worker, WorkerClient};
use zkpd::p2p::scalar_worker::{
    parse_peer, ExampleWorker, ExampleWorkerClient, Packet, ReceiveShareResponse,
    SendShareResponse, SetPeerWorkersResponse, WorkResponse,
};

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

                let response: Packet<Bls381K12Scalar> = serde_json::from_str(&text).unwrap();
                match response {
                    Packet::SetPeerWorkersRequest(req) => {
                        let workers = req
                            .peers
                            .iter()
                            .map(|peer| {
                                let (id, url) = parse_peer(peer);
                                Arc::new(ExampleWorkerClient::<Bls381K12Scalar>::new(id, url))
                                    as Arc<dyn WorkerClient<Bls381K12Scalar>>
                            })
                            .collect();
                        let mut peer_workers = w.peer_workers.lock().unwrap();
                        *peer_workers = workers;
                        println!("Set peer workers: {:?}", req.peers);
                        let response = Packet::<Bls381K12Scalar>::SetPeerWorkersResponse(
                            SetPeerWorkersResponse {},
                        );
                        let encoded = serde_json::to_string(&response).unwrap();
                        websocket.send(Message::Text(encoded)).unwrap();
                        websocket.flush().unwrap();
                    }
                    Packet::SendShareRequest(req) => {
                        w.insert_share(req.stage, req.from_worker, req.a_b_share_shifted);
                        let response =
                            Packet::<Bls381K12Scalar>::SendShareResponse(SendShareResponse {});
                        let encoded = serde_json::to_string(&response).unwrap();
                        websocket.send(Message::Text(encoded)).unwrap();
                        websocket.flush().unwrap();
                    }
                    Packet::WorkRequest(req) => {
                        let shares = w.work(req.beaver_triple_shares, req.input_shares);
                        let response = Packet::<Bls381K12Scalar>::WorkResponse(WorkResponse::<
                            Bls381K12Scalar,
                        > {
                            shares,
                        });
                        let encoded = serde_json::to_string(&response).unwrap();
                        websocket.send(Message::Text(encoded)).unwrap();
                        websocket.flush().unwrap();
                    }
                    Packet::ReceiveShareRequest(req) => {
                        let a_b_share_shifted = w.get_share(req.stage, w.index());
                        let response =
                            Packet::<Bls381K12Scalar>::ReceiveShareResponse(ReceiveShareResponse {
                                a_b_share_shifted,
                            });
                        let encoded = serde_json::to_string(&response).unwrap();
                        websocket.send(Message::Text(encoded)).unwrap();
                        websocket.flush().unwrap();
                    }
                    _ => {
                        println!("Unexpected message.");
                    }
                }
            }
            Message::Close(_) => {
                println!("Connection closed.");
                break;
            }
            _ => {
                println!("Unexpected message, quit.");
                break;
            }
        }
    }
}
