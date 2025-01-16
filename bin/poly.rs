use rayon::prelude::*;
use std::collections::HashMap;
use std::iter;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::secret_sharing::SecretSharing as SecretSharingImpl;
use zkpd::util::naive_mul;
use zkpd::{
    beaver_triple_generatoor::BeaverTripleGeneratoor as BeaverTripleGeneratoorImpl,
    BeaverTripleGeneratoor, FiniteField, SecretSharing,
};

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
        let workers = self.workers.len();
        let product_coeffs = poly1.len() + poly2.len() - 1;
        let random_shares = setup_random_shares(workers, product_coeffs);

        let mut poly_shares1 = vec![];
        for _i in 0..workers {
            poly_shares1.push(vec![]);
        }
        for coeff in poly1 {
            let shares = SecretSharingImpl::share(coeff, workers, workers);
            for i in 0..workers {
                poly_shares1[i].push(shares[i].clone());
            }
        }
        let mut poly_shares2 = vec![];
        for _i in 0..workers {
            poly_shares2.push(vec![]);
        }
        for coeff in poly2 {
            let shares = SecretSharingImpl::share(coeff, workers, workers);
            for i in 0..workers {
                poly_shares2[i].push(shares[i].clone());
            }
        }
        let output_shares: Vec<Vec<Bls381K12Scalar>> = self
            .workers
            .par_iter()
            .map(|worker| {
                let idx = worker.index() - 1;
                worker.work(
                    random_shares[idx].clone(),
                    poly_shares1[idx].clone(),
                    poly_shares2[idx].clone(),
                )
            })
            .collect();

        assert!(output_shares.len() == workers, "output_shares size wrong");

        (0..product_coeffs)
            .map(|coeff_index| {
                let coeff_shares: Vec<Bls381K12Scalar> = (0..workers)
                    .map(|w| output_shares[w][coeff_index])
                    .collect();
                SecretSharingImpl::recover(
                    coeff_shares,
                    (self.workers.iter().map(|w| w.index())).collect(),
                    self.workers.len(),
                    self.workers.len(),
                )
            })
            .collect()
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

fn setup_random_shares(
    workers: usize,
    product_coeffs: usize,
) -> Vec<Vec<(Bls381K12Scalar, Bls381K12Scalar, Bls381K12Scalar)>> {
    let rs: Vec<(Bls381K12Scalar, Bls381K12Scalar, Bls381K12Scalar)> = (0..product_coeffs)
        .map(|_| BeaverTripleGeneratoorImpl::<Bls381K12Scalar>::generate())
        .collect();

    let mut result = vec![];
    for _i in 0..workers {
        result.push(vec![]);
    }

    for r in rs {
        let alpha_shares = SecretSharingImpl::share(r.0, workers, workers);
        let beta_shares = SecretSharingImpl::share(r.1, workers, workers);
        let gama_shares = SecretSharingImpl::share(r.2, workers, workers);

        for i in 0..workers {
            result[i].push((alpha_shares[i], beta_shares[i], gama_shares[i]));
        }
    }

    result
}
struct ExampleWorker<T: FiniteField> {
    index: usize,
    peer_workers: Mutex<Vec<Arc<dyn WorkerClient<T>>>>,
    stage_shares: Mutex<Vec<HashMap<usize, Vec<(T, T)>>>>,
}

impl<T: FiniteField> ExampleWorker<T> {
    fn insert_share(&self, stage: usize, from_worker: usize, a_b_share_shifted: Vec<(T, T)>) {
        let mut stage_shares = self.stage_shares.lock().unwrap();
        if stage_shares.len() == stage {
            stage_shares.push(HashMap::new());
        } else if stage_shares.len() == stage + 1 {
        } else {
            panic!("invalid stage");
        }
        stage_shares[stage].insert(from_worker, a_b_share_shifted);
    }

    fn get_share(&self, stage: usize, from_worker: usize) -> Option<Vec<(T, T)>> {
        let stage_shares = self.stage_shares.lock().unwrap();
        stage_shares[stage].get(&from_worker).map(|x| x.clone())
    }
}

impl<T: FiniteField> Base<T> for ExampleWorker<T> {
    fn index(&self) -> usize {
        self.index
    }
    fn work(
        &self,
        beaver_triple_shares: Vec<(T, T, T)>,
        poly1_shares: Vec<T>,
        poly2_shares: Vec<T>,
    ) -> Vec<T> {
        self.multiply_poly(0, poly1_shares, poly2_shares, &beaver_triple_shares)
    }
}

impl<T: FiniteField> Worker<T> for ExampleWorker<T> {
    fn broadcast_poly(&self, a_b_share_shifted: Vec<(T, T)>, stage: usize) {
        self.insert_share(stage, self.index, a_b_share_shifted.clone());
        let peer_workers = self.peer_workers.lock().unwrap();

        for w in peer_workers.iter() {
            w.send_share(self.index(), a_b_share_shifted.clone(), stage);
        }
    }
    fn wait_for_broadcast_poly(&self, stage: usize) -> Vec<(T, T)> {
        let mut all_shares = vec![self.get_share(stage, self.index).unwrap()];
        let peer_workers = self.peer_workers.lock().unwrap();
        for w in peer_workers.iter() {
            all_shares.push(w.receive_share(stage));
        }
        let all_indices: Vec<_> = iter::once(self.index)
            .chain(peer_workers.iter().map(|w| w.index()))
            .collect();
        let n = 1 + peer_workers.len();
        let mut result = vec![];
        for i in 0..all_shares[0].len() {
            let mut shifted_a_shares = vec![];
            let mut shifted_b_shares = vec![];
            for j in 0..n {
                let ab_shares_from_j = &all_shares[j];
                shifted_a_shares.push(ab_shares_from_j[i].0.clone());
                shifted_b_shares.push(ab_shares_from_j[i].1.clone());
            }
            result.push((
                SecretSharingImpl::recover(shifted_a_shares, all_indices.clone(), n, n),
                SecretSharingImpl::recover(shifted_b_shares, all_indices.clone(), n, n),
            ));
        }
        result
    }
}

struct ExampleWorkerClient<T: FiniteField> {
    worker: Arc<ExampleWorker<T>>,
}

impl<'a, T: FiniteField> Base<T> for ExampleWorkerClient<T> {
    fn index(&self) -> usize {
        self.worker.index
    }
    fn work(
        &self,
        beaver_triple_shares: Vec<(T, T, T)>,
        poly1_shares: Vec<T>,
        poly2_shares: Vec<T>,
    ) -> Vec<T> {
        self.worker
            .work(beaver_triple_shares, poly1_shares, poly2_shares)
    }
}

impl<T: FiniteField> WorkerClient<T> for ExampleWorkerClient<T> {
    fn set_peer_workers(&self, _peer_workers: Vec<Arc<dyn WorkerClient<T>>>) {
        let mut peer_workers = self.worker.peer_workers.lock().unwrap();
        *peer_workers = _peer_workers;
    }

    fn send_share(&self, from_worker: usize, a_b_share_shifted: Vec<(T, T)>, stage: usize) {
        self.worker
            .insert_share(stage, from_worker, a_b_share_shifted);
    }

    fn receive_share(&self, stage: usize) -> Vec<(T, T)> {
        loop {
            let stage_shares = self.worker.stage_shares.lock().unwrap();
            if stage_shares.len() != stage + 1 {
                println!("waiting for stage {}", stage);
                drop(stage_shares);
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            if !stage_shares[stage].contains_key(&self.worker.index) {
                println!(
                    "waiting for stage {} from worker {}",
                    stage, self.worker.index
                );
                drop(stage_shares);
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            let share = stage_shares[stage].get(&self.worker.index).unwrap();
            return share.clone();
        }
    }
}

fn main() {
    let n = 10;
    let rand_poly1: Vec<Bls381K12Scalar> = (0..n).map(|_| Bls381K12Scalar::random()).collect();
    let rand_poly2: Vec<Bls381K12Scalar> = (0..n).map(|_| Bls381K12Scalar::random()).collect();

    let expected = naive_mul(&rand_poly1, &rand_poly2);

    let w1 = ExampleWorker::<Bls381K12Scalar> {
        index: 1,
        peer_workers: Mutex::new(vec![]),
        stage_shares: Mutex::new(vec![]),
    };
    let w2 = ExampleWorker::<Bls381K12Scalar> {
        index: 2,
        peer_workers: Mutex::new(vec![]),
        stage_shares: Mutex::new(vec![]),
    };
    let w3 = ExampleWorker::<Bls381K12Scalar> {
        index: 3,
        peer_workers: Mutex::new(vec![]),
        stage_shares: Mutex::new(vec![]),
    };
    let c1 = ExampleWorkerClient::<Bls381K12Scalar> {
        worker: Arc::new(w1),
    };
    let c2 = ExampleWorkerClient::<Bls381K12Scalar> {
        worker: Arc::new(w2),
    };
    let c3 = ExampleWorkerClient::<Bls381K12Scalar> {
        worker: Arc::new(w3),
    };

    let worker_clients: Vec<Arc<dyn WorkerClient<Bls381K12Scalar>>> =
        vec![Arc::new(c1), Arc::new(c2), Arc::new(c3)];

    let d = ExampleDelegator::<Bls381K12Scalar>::new(worker_clients);

    let result: Vec<Bls381K12Scalar> = d.delegate(rand_poly1, rand_poly2);
    println!("result:{:?}, expected:{:?}", result, expected);
    assert!(result == expected);
    println!("recovered == expected");
}
