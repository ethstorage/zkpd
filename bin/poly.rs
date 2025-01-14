use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::util::naive_mul;
use zkpd::FiniteField;

use zkpd::mode::poly::{Base, Delegator, Worker, WorkerClient};

struct ExampleDelegator<T: FiniteField> {
    workers: Vec<Arc<dyn WorkerClient<T>>>,
}

impl Delegator<Bls381K12Scalar> for ExampleDelegator<Bls381K12Scalar> {
    fn delegate(
        &self,
        poly1: Vec<Bls381K12Scalar>,
        poly2: Vec<Bls381K12Scalar>,
    ) -> Vec<Bls381K12Scalar> {
        vec![]
    }

    fn new(workers: Vec<Arc<dyn WorkerClient<Bls381K12Scalar>>>) -> Self {
        for w in workers.iter() {
            let mut peer_workers = workers.clone();
            peer_workers.retain(|x| x.index() != w.index());
            w.set_peer_workers(peer_workers);
        }
        ExampleDelegator { workers }
    }
}
struct ExampleWorker<T: FiniteField> {
    index: usize,
    peer_workers: Mutex<Vec<Arc<dyn WorkerClient<T>>>>,
    stage_shares: Mutex<Vec<HashMap<usize, (T, T)>>>,
}

struct ExampleWorkerClient<T: FiniteField> {
    worker: Arc<ExampleWorker<T>>,
}

fn main() {
    let n = 10;
    let rand_poly1: Vec<Bls381K12Scalar> = (0..n).map(|_| Bls381K12Scalar::random()).collect();
    let rand_poly2: Vec<Bls381K12Scalar> = (0..n).map(|_| Bls381K12Scalar::random()).collect();

    let expected = naive_mul(&rand_poly1, &rand_poly2);

    let result: Vec<Bls381K12Scalar> = vec![];
    println!("result:{:?}, expected:{:?}", result, expected);
    assert!(result == expected);
}
