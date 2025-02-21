use core::panic;
use std::collections::HashMap;
use std::iter;
use std::sync::{Arc, Mutex};

use crate::mode::scalar::{Base, Worker, WorkerClient};
use crate::secret_sharing::SecretSharing as SecretSharingImpl;
use crate::{FiniteField, SecretSharing};

use tokio::net::TcpStream;

use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

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

pub struct ExampleWorkerClient<T: FiniteField> {
    _marker: PhantomData<T>,
    index: usize,
    url: String,
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl<T: FiniteField> ExampleWorkerClient<T> {
    pub async fn new(index: usize, url: String) -> Self {
        let ws_stream = tokio_tungstenite::connect_async(url.clone())
            .await
            .unwrap()
            .0;
        ExampleWorkerClient {
            _marker: PhantomData,
            index,
            url,
            ws_stream,
        }
    }

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
