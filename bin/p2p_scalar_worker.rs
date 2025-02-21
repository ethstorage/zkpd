use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration as TokioDuration};
use tokio_tungstenite::accept_async;
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let addr = format!("127.0.0.1:{}", args.port);
    println!("WebSocket server listening on: {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("Sleep 2s before connecting peers");
    sleep(TokioDuration::from_secs(2)).await;

    let w = Arc::new(ExampleWorker::<Bls381K12Scalar> {
        index: args.index,
        peer_workers: Mutex::new(vec![]),
        stage_shares: Mutex::new(vec![]),
    });

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream)
            .await
            .expect("Error during WebSocket handshake");

        tokio::spawn(handle_connection(w.clone(), ws_stream));
    }
}

async fn handle_connection(
    w: Arc<ExampleWorker<Bls381K12Scalar>>,
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(msg) => {
                println!("Received: {}", msg);

                // Echo the message back
                if let Err(e) = ws_sender.send(msg).await {
                    eprintln!("Error sending message: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                break;
            }
        }
    }
}
