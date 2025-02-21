use core::panic;
use std::collections::HashMap;
use std::iter;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use clap::Parser;
use futures_util::sink::SinkExt;
use futures_util::stream::{self, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task;
use tokio::time::{sleep, Duration as TokioDuration};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;
use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::mode::scalar::{Base, Worker, WorkerClient};
use zkpd::secret_sharing::SecretSharing as SecretSharingImpl;
use zkpd::{FiniteField, SecretSharing};

#[derive(Parser, Debug)]
struct Args {
    /// The index of worker
    #[arg(long)]
    index: usize,

    /// The listening port
    #[arg(long)]
    port: u16,
}

fn parse_peer(peer: &str) -> (usize, String) {
    let parts: Vec<&str> = peer.split('@').collect();
    if parts.len() != 2 {
        panic!("invalid peer format");
    }
    let id = parts[0].parse::<usize>().unwrap();
    let peer = parts[1].to_string();
    (id, peer)
}

struct ExampleWorker<T: FiniteField> {
    index: usize,
    peer_workers: Mutex<Vec<Arc<dyn WorkerClient<T>>>>,
    stage_shares: Mutex<Vec<HashMap<usize, (T, T)>>>,
}

impl<T: FiniteField> ExampleWorker<T> {
    fn insert_share(&self, stage: usize, from_worker: usize, a_b_share_shifted: (T, T)) {
        let mut stage_shares = self.stage_shares.lock().unwrap();
        if stage_shares.len() == stage {
            stage_shares.push(HashMap::new());
        } else if stage_shares.len() == stage + 1 {
        } else {
            panic!("invalid stage");
        }
        stage_shares[stage].insert(from_worker, a_b_share_shifted);
    }

    fn get_share(&self, stage: usize, from_worker: usize) -> Option<(T, T)> {
        let stage_shares = self.stage_shares.lock().unwrap();
        stage_shares[stage]
            .get(&from_worker)
            .map(|x| (x.0.clone(), x.1.clone()))
    }
}

impl<T: FiniteField> Base<T> for ExampleWorker<T> {
    fn index(&self) -> usize {
        self.index
    }

    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T> {
        let x_2 = self.multiply(
            0,
            input_shares[0].clone(),
            input_shares[0].clone(),
            &beaver_triple_shares[0],
        );

        let x_3 = self.multiply(
            1,
            x_2.clone(),
            input_shares[0].clone(),
            &beaver_triple_shares[1],
        );
        let target = x_3
            .add(&x_2.mul(&T::from_usize(5)))
            .add(&input_shares[0].clone().mul(&T::from_usize(3)))
            .add(&T::from_usize(2));
        vec![target]
    }
}

impl<T: FiniteField> Worker<T> for ExampleWorker<T> {
    fn broadcast(&self, a_b_share_shifted: (T, T), stage: usize) {
        self.insert_share(
            stage,
            self.index,
            (a_b_share_shifted.0.clone(), a_b_share_shifted.1.clone()),
        );
        let peer_workers = self.peer_workers.lock().unwrap();

        for w in peer_workers.iter() {
            w.send_share(
                self.index(),
                (a_b_share_shifted.0.clone(), a_b_share_shifted.1.clone()),
                stage,
            );
        }
    }
    fn wait_for_broadcast(&self, stage: usize) -> (T, T) {
        let self_shares = self.get_share(stage, self.index).unwrap();
        let peer_workers = self.peer_workers.lock().unwrap();
        let mut a_shares_shifted = vec![self_shares.0];
        let mut b_shares_shifted = vec![self_shares.1];
        for w in peer_workers.iter() {
            let (a_share_shifted, b_share_shifted) = w.receive_share(stage);
            a_shares_shifted.push(a_share_shifted);
            b_shares_shifted.push(b_share_shifted);
        }
        let indices: Vec<_> = iter::once(self.index)
            .chain(peer_workers.iter().map(|w| w.index()))
            .collect();
        let n = 1 + peer_workers.len();
        (
            SecretSharingImpl::recover(a_shares_shifted, indices.clone(), n, n),
            SecretSharingImpl::recover(b_shares_shifted, indices, n, n),
        )
    }
}

use std::marker::PhantomData;

struct ExampleWorkerClient<T: FiniteField> {
    _marker: PhantomData<T>,
    index: usize,
    peer: String,
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl<T: FiniteField> ExampleWorkerClient<T> {
    fn set_peer_workers(&self, peer_workers: Vec<Arc<dyn WorkerClient<T>>>) {}

    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T> {
        tokio::runtime::Handle::current().block_on(async {
            // self.ws_stream.work(beaver_triple_shares, input_shares);
            vec![]
        })
    }

    fn send_share(&self, from_worker: usize, a_b_share_shifted: (T, T), stage: usize) {}

    fn receive_share(&self, stage: usize) -> (T, T) {
        (T::zero(), T::zero())
    }
}

impl<'a, T: FiniteField> Base<T> for ExampleWorkerClient<T> {
    fn index(&self) -> usize {
        self.index
    }
    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T> {
        ExampleWorkerClient::work(self, beaver_triple_shares, input_shares)
    }
}

impl<T: FiniteField> WorkerClient<T> for ExampleWorkerClient<T> {
    fn set_peer_workers(&self, _peer_workers: Vec<Arc<dyn WorkerClient<T>>>) {
        ExampleWorkerClient::set_peer_workers(self, _peer_workers);
    }

    fn send_share(&self, from_worker: usize, a_b_share_shifted: (T, T), stage: usize) {
        ExampleWorkerClient::send_share(self, from_worker, a_b_share_shifted, stage);
    }

    fn receive_share(&self, stage: usize) -> (T, T) {
        ExampleWorkerClient::receive_share(&self, stage)
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let addr = format!("127.0.0.1:{}", args.port);
    println!("WebSocket server listening on: {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("Sleep 2s before connecting peers");
    sleep(Duration::from_secs(2)).await;

    // let peer_workers = stream::iter(args.peer_workers)
    //     .then(|peer| async {
    //         let (peer_id, peer_addr) = parse_peer(&peer);
    //         let (ws_stream, _) = connect_async(Url::parse(&peer_addr).unwrap())
    //             .await
    //             .expect("Failed to connect to peer");
    //         Arc::new(ExampleWorkerClient::<Bls381K12Scalar> {
    //             _marker: PhantomData,
    //             index: peer_id,
    //             peer: peer,
    //             ws_stream: ws_stream,
    //         }) as Arc<dyn WorkerClient<Bls381K12Scalar>>
    //     })
    //     .collect()
    //     .await;

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
