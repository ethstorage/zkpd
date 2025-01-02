use std::sync::{Arc, Mutex};

use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::secret_sharing::SecretSharing as SecretSharingImpl;
use zkpd::{
    beaver_triple_generatoor::BeaverTripleGeneratoor as BeaverTripleGeneratoorImpl,
    BeaverTripleGeneratoor, FiniteField, SecretSharing,
};
use zkpd::{Delegator, Worker};

struct ExampleDelegator<T: FiniteField> {
    _marker: std::marker::PhantomData<T>,
    workers: Vec<Arc<Box<dyn Worker<T>>>>,
}

impl Delegator<Bls381K12Scalar> for ExampleDelegator<Bls381K12Scalar> {
    fn new(workers: Vec<Arc<Box<dyn Worker<Bls381K12Scalar>>>>) -> Self {
        for w in workers.iter() {
            let mut peer_workers = workers.clone();
            peer_workers.retain(|x| x.index() != w.index());
            w.set_peer_workers(peer_workers);
        }
        ExampleDelegator {
            _marker: std::marker::PhantomData,
            workers,
        }
    }
    fn delegate(&self, inputs: Vec<Bls381K12Scalar>) -> Vec<Bls381K12Scalar> {
        assert!(inputs.len() == 1);
        let random_shares = setup_random_shares(self.workers.len());

        let input_shares =
            SecretSharingImpl::share(inputs[0], self.workers.len(), self.workers.len());
        for worker in self.workers.iter() {
            let idx = worker.index() - 1;
            worker.work(random_shares[idx].clone(), vec![input_shares[idx]]);
        }
        vec![]
    }
}

fn setup_random_shares(n: usize) -> Vec<Vec<(Bls381K12Scalar, Bls381K12Scalar, Bls381K12Scalar)>> {
    let r1 = BeaverTripleGeneratoorImpl::<Bls381K12Scalar>::generate();
    let r2 = BeaverTripleGeneratoorImpl::<Bls381K12Scalar>::generate();
    let r3 = BeaverTripleGeneratoorImpl::<Bls381K12Scalar>::generate();
    let rs = vec![r1, r2, r3];

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

struct ExampleWorker<T: FiniteField> {
    _marker: std::marker::PhantomData<T>,
    index: usize,
    peer_workers: Mutex<Vec<Arc<Box<dyn Worker<T>>>>>,
}

impl<T: FiniteField> Worker<T> for ExampleWorker<T> {
    fn index(&self) -> usize {
        self.index
    }
    fn set_peer_workers(&self, peer_workers: Vec<Arc<Box<dyn Worker<T>>>>) {
        let mut _peer_workers = self.peer_workers.lock().unwrap();
        *_peer_workers = peer_workers;
    }

    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T> {
        vec![]
    }
    fn broadcast(&self, a_share_shifted: T, b_share_shifted: T, stage: usize) {
        let peer_workers = self.peer_workers.lock().unwrap();
        for w in peer_workers.iter() {
            w.broadcast(a_share_shifted.clone(), b_share_shifted.clone(), stage);
        }
    }
    fn wait_for_broadcast(&self) -> (T, T) {
        let peer_workers = self.peer_workers.lock().unwrap();
        let mut sum_a_share_shifted = T::zero();
        let mut sum_b_share_shifted = T::zero();
        for w in peer_workers.iter() {
            let (a_share_shifted, b_share_shifted) = w.wait_for_broadcast();
            sum_a_share_shifted = sum_a_share_shifted.add(&a_share_shifted);
            sum_b_share_shifted = sum_b_share_shifted.add(&b_share_shifted);
        }
        (sum_a_share_shifted, sum_b_share_shifted)
    }
}

fn main() {
    let x = Bls381K12Scalar::from_usize(100);

    let expected = x * x * x
        + Bls381K12Scalar::from_usize(5) * x * x
        + Bls381K12Scalar::from_usize(3) * x
        + Bls381K12Scalar::from_usize(2);

    // zkpd for x^3 + 5x^2 + 3x + 2

    let w1: Box<dyn Worker<Bls381K12Scalar>> = Box::new(ExampleWorker::<Bls381K12Scalar> {
        _marker: std::marker::PhantomData,
        index: 1,
        peer_workers: Mutex::new(vec![]),
    });
    let w2: Box<dyn Worker<Bls381K12Scalar>> = Box::new(ExampleWorker::<Bls381K12Scalar> {
        _marker: std::marker::PhantomData,
        index: 2,
        peer_workers: Mutex::new(vec![]),
    });
    let workers = vec![Arc::new(w1), Arc::new(w2)];
    let d = ExampleDelegator::<Bls381K12Scalar>::new(workers);

    let result = d.delegate(vec![x]);

    assert!(result.len() == 1 && result[0] == expected);
}
