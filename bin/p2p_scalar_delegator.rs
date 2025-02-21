use core::panic;
use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;
use std::collections::HashMap;
use std::iter;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use clap::Parser;
use futures_util::stream::{self, StreamExt};
use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::mode::scalar::{Base, Delegator, Worker, WorkerClient};
use zkpd::p2p::scalar_worker::{parse_peer, ExampleWorkerClient};
use zkpd::secret_sharing::SecretSharing as SecretSharingImpl;
use zkpd::{
    beaver_triple_generatoor::BeaverTripleGeneratoor as BeaverTripleGeneratoorImpl,
    BeaverTripleGeneratoor, FiniteField, SecretSharing,
};

#[derive(Parser, Debug)]
struct Args {
    /// all workers
    #[arg(long,num_args = 1..)]
    workers: Vec<String>,
}

struct ExampleDelegator<T: FiniteField> {
    workers: Vec<Arc<dyn WorkerClient<T>>>,
}

impl Delegator<Bls381K12Scalar> for ExampleDelegator<Bls381K12Scalar> {
    fn new(workers: Vec<Arc<dyn WorkerClient<Bls381K12Scalar>>>) -> Self {
        for w in workers.iter() {
            let mut peer_workers = workers.clone();
            peer_workers.retain(|x| x.index() != w.index());
            w.set_peer_workers(peer_workers);
        }
        ExampleDelegator { workers }
    }
    fn delegate(&self, inputs: Vec<Bls381K12Scalar>) -> Vec<Bls381K12Scalar> {
        assert!(inputs.len() == 1);
        let random_shares = setup_random_shares(self.workers.len());

        let input_shares =
            SecretSharingImpl::share(inputs[0], self.workers.len(), self.workers.len());

        let output_shares = self
            .workers
            .par_iter()
            .map(|worker| {
                let idx = worker.index() - 1;
                worker.work(random_shares[idx].clone(), vec![input_shares[idx].clone()])[0]
            })
            .collect();

        vec![SecretSharingImpl::recover(
            output_shares,
            (self.workers.iter().map(|w| w.index())).collect(),
            self.workers.len(),
            self.workers.len(),
        )]
    }
}

fn setup_random_shares(n: usize) -> Vec<Vec<(Bls381K12Scalar, Bls381K12Scalar, Bls381K12Scalar)>> {
    let r1 = BeaverTripleGeneratoorImpl::<Bls381K12Scalar>::generate();
    let r2 = BeaverTripleGeneratoorImpl::<Bls381K12Scalar>::generate();
    let rs = vec![r1, r2];

    let mut result = vec![];
    for _i in 0..n {
        result.push(vec![]);
    }
    for r in rs {
        let alpha_shares = SecretSharingImpl::share(r.0, n, n);
        let beta_shares = SecretSharingImpl::share(r.1, n, n);
        let gama_shares = SecretSharingImpl::share(r.2, n, n);

        for i in 0..n {
            result[i].push((alpha_shares[i], beta_shares[i], gama_shares[i]));
        }
    }

    result
}

#[tokio::main]
async fn main() {
    let x = Bls381K12Scalar::from_usize(100);

    let expected = x * x * x
        + Bls381K12Scalar::from_usize(5) * x * x
        + Bls381K12Scalar::from_usize(3) * x
        + Bls381K12Scalar::from_usize(2);

    // zkpd for x^3 + 5x^2 + 3x + 2

    let args = Args::parse();

    let worker_clients = stream::iter(args.workers)
        .then(|peer| async move {
            let (id, url) = parse_peer(&peer);
            Arc::new(ExampleWorkerClient::<Bls381K12Scalar>::new(id, url).await)
                as Arc<dyn WorkerClient<Bls381K12Scalar>>
        })
        .collect()
        .await;

    let d = ExampleDelegator::<Bls381K12Scalar>::new(worker_clients);

    let result = d.delegate(vec![x]);
    println!("result:{:?}, expected:{:?}", result, expected);
    assert!(result.len() == 1 && result[0] == expected);
}
