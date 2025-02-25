use core::panic;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{iter, thread};

use crate::mode::scalar::{Base, Worker, WorkerClient};
use crate::secret_sharing::SecretSharing as SecretSharingImpl;
use crate::{FiniteField, SecretSharing};

use serde::{Deserialize, Serialize};
use std::net::TcpStream;

use std::time::Duration;
use tungstenite::protocol::WebSocket;
use tungstenite::{connect, stream::MaybeTlsStream, Message};

pub fn parse_peer(peer: &str) -> (usize, String) {
    let parts: Vec<&str> = peer.split('@').collect();
    if parts.len() != 2 {
        panic!("invalid peer format");
    }
    let id = parts[0].parse::<usize>().unwrap();
    let peer = parts[1].to_string();
    (id, peer)
}

pub struct ExampleWorker<T: FiniteField> {
    pub index: usize,
    pub peer_workers: Mutex<Vec<Arc<dyn WorkerClient<T>>>>,
    pub stage_shares: Mutex<Vec<HashMap<usize, (T, T)>>>,
}

impl<T: FiniteField> ExampleWorker<T> {
    pub fn insert_share(&self, stage: usize, from_worker: usize, a_b_share_shifted: (T, T)) {
        let mut stage_shares = self.stage_shares.lock().unwrap();
        if stage_shares.len() == stage {
            stage_shares.push(HashMap::new());
        } else if stage_shares.len() == stage + 1 {
        } else {
            panic!("invalid stage");
        }
        stage_shares[stage].insert(from_worker, a_b_share_shifted);
    }

    pub fn get_share(&self, stage: usize, from_worker: usize) -> Option<(T, T)> {
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

#[derive(Serialize, Deserialize, Debug)]
pub enum Packet<T> {
    SetPeerWorkersRequest(SetPeerWorkersRequest),
    SetPeerWorkersResponse(SetPeerWorkersResponse),
    WorkRequest(WorkRequest<T>),
    WorkResponse(WorkResponse<T>),
    SendShareRequest(SendShareRequest<T>),
    SendShareResponse(SendShareResponse),
    ReceiveShareRequest(ReceiveShareRequest),
    ReceiveShareResponse(ReceiveShareResponse<T>),
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SetPeerWorkersRequest {
    pub peers: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetPeerWorkersResponse {}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkRequest<T> {
    pub beaver_triple_shares: Vec<(T, T, T)>,
    pub input_shares: Vec<T>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkResponse<T> {
    pub shares: Vec<T>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendShareRequest<T> {
    pub from_worker: usize,
    pub a_b_share_shifted: (T, T),
    pub stage: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendShareResponse {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReceiveShareRequest {
    pub stage: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReceiveShareResponse<T> {
    pub a_b_share_shifted: Option<(T, T)>,
}

pub struct ExampleWorkerClient<T: FiniteField> {
    _marker: PhantomData<T>,
    index: usize,
    url: String,
    socket: Mutex<WebSocket<MaybeTlsStream<TcpStream>>>,
}

impl<T: FiniteField> ExampleWorkerClient<T> {
    pub fn new(index: usize, url: String) -> Self {
        let socket = connect(url.clone()).unwrap().0;
        ExampleWorkerClient {
            _marker: PhantomData,
            index,
            url,
            socket: Mutex::new(socket),
        }
    }

    fn set_peer_workers(&self, peer_workers: Vec<Arc<dyn WorkerClient<T>>>) {
        let peers = peer_workers
            .iter()
            .map(|w| {
                let client = (w as &dyn Any)
                    .downcast_ref::<ExampleWorkerClient<T>>()
                    .unwrap();
                format!("{}@{}", w.index(), client.url)
            })
            .collect();

        let encoded =
            serde_json::to_string(&Packet::SetPeerWorkersRequest::<T>(SetPeerWorkersRequest {
                peers,
            }))
            .unwrap();

        let mut socket = self.socket.lock().unwrap();
        socket.write(Message::Text(encoded)).unwrap();
        let msg = socket.read().unwrap();
        let response: Packet<T> = serde_json::from_str(&msg.to_text().unwrap()).unwrap();
        match response {
            Packet::SetPeerWorkersResponse(_) => return (),
            _ => panic!("unexpected response"),
        };
    }

    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T> {
        let encoded = serde_json::to_string(&Packet::WorkRequest(WorkRequest {
            beaver_triple_shares: beaver_triple_shares,
            input_shares: input_shares,
        }))
        .unwrap();

        let mut socket = self.socket.lock().unwrap();
        socket.write(Message::Text(encoded)).unwrap();
        let msg = socket.read().unwrap();
        let response: Packet<T> = serde_json::from_str(&msg.to_text().unwrap()).unwrap();
        match response {
            Packet::WorkResponse(WorkResponse { shares }) => return shares,
            _ => panic!("unexpected response"),
        };
    }

    fn send_share(&self, from_worker: usize, a_b_share_shifted: (T, T), stage: usize) {
        let encoded = serde_json::to_string(&Packet::SendShareRequest(SendShareRequest {
            from_worker,
            a_b_share_shifted,
            stage,
        }))
        .unwrap();

        let mut socket = self.socket.lock().unwrap();
        socket.write(Message::Text(encoded)).unwrap();
        let msg = socket.read().unwrap();
        let response: Packet<T> = serde_json::from_str(&msg.to_text().unwrap()).unwrap();
        match response {
            Packet::SendShareResponse(SendShareResponse {}) => return (),
            _ => panic!("unexpected response"),
        };
    }

    fn receive_share(&self, stage: usize) -> (T, T) {
        let mut socket = self.socket.lock().unwrap();

        loop {
            let encoded =
                serde_json::to_string(&Packet::<T>::ReceiveShareRequest(ReceiveShareRequest {
                    stage,
                }))
                .unwrap();
            socket.write(Message::Text(encoded)).unwrap();
            let msg = socket.read().unwrap();
            let response: Packet<T> = serde_json::from_str(&msg.to_text().unwrap()).unwrap();
            match response {
                Packet::ReceiveShareResponse(ReceiveShareResponse { a_b_share_shifted }) => {
                    if a_b_share_shifted.is_some() {
                        return a_b_share_shifted.unwrap();
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                _ => panic!("unexpected response"),
            };
        }
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
