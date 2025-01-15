use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::secret_sharing::SecretSharing as SecretSharingImpl;
use zkpd::util::naive_mul;
use zkpd::{
    beaver_triple_generatoor::BeaverTripleGeneratoor as BeaverTripleGeneratoorImpl,
    BeaverTripleGeneratoor, FiniteField, SecretSharing,
};

use zkpd::mode::poly::{self, Base, Delegator, Worker, WorkerClient};

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
    stage_shares: Mutex<Vec<HashMap<usize, (T, T)>>>,
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
    fn broadcast_poly(&self, _a_b_share_shifted: Vec<(T, T)>, _stage: usize) {}
    fn wait_for_broadcast_poly(&self, _stage: usize) -> Vec<(T, T)> {
        vec![]
    }
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
