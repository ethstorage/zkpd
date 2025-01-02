use core::panic;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::secret_sharing::SecretSharing as SecretSharingImpl;
use zkpd::{
    beaver_triple_generatoor::BeaverTripleGeneratoor as BeaverTripleGeneratoorImpl,
    BeaverTripleGeneratoor, FiniteField, SecretSharing,
};
use zkpd::{Base, Delegator, Worker, WorkerClient};

struct ExampleDelegator<T: FiniteField> {
    _marker: std::marker::PhantomData<T>,
    workers: Vec<Arc<dyn WorkerClient<T>>>,
}

impl Delegator<Bls381K12Scalar> for ExampleDelegator<Bls381K12Scalar> {
    fn new(workers: Vec<Arc<dyn WorkerClient<Bls381K12Scalar>>>) -> Self {
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
        let mut output_shares = vec![];
        for worker in self.workers.iter() {
            let idx = worker.index() - 1;
            output_shares.push(worker.work(random_shares[idx].clone(), vec![input_shares[idx]])[0]);
        }
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
    peer_workers: Mutex<Vec<Arc<Box<dyn WorkerClient<T>>>>>,
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
        let peer_workers = self.peer_workers.lock().unwrap();
        let mut sum_a_share_shifted = T::zero();
        let mut sum_b_share_shifted = T::zero();
        for w in peer_workers.iter() {
            let (a_share_shifted, b_share_shifted) = w.receive_share(stage);
            sum_a_share_shifted = sum_a_share_shifted.add(&a_share_shifted);
            sum_b_share_shifted = sum_b_share_shifted.add(&b_share_shifted);
        }
        (sum_a_share_shifted, sum_b_share_shifted)
    }
}

struct ExampleWorkerClient<T: FiniteField> {
    _marker: std::marker::PhantomData<T>,
    worker: Arc<ExampleWorker<T>>,
}

impl<'a, T: FiniteField> Base<T> for ExampleWorkerClient<T> {
    fn index(&self) -> usize {
        self.worker.index
    }
    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T> {
        self.worker.work(beaver_triple_shares, input_shares)
    }
}

impl<T: FiniteField> WorkerClient<T> for ExampleWorkerClient<T> {
    fn set_peer_workers(&self, peer_workers: Vec<Arc<dyn WorkerClient<T>>>) {
        let mut peer_workers = peer_workers;
        peer_workers.retain(|x| x.index() != self.worker.index());
        let mut peer_workers = self.worker.peer_workers.lock().unwrap();
        *peer_workers = peer_workers.clone();
    }

    fn send_share(&self, from_worker: usize, a_b_share_shifted: (T, T), stage: usize) {
        self.worker
            .insert_share(stage, from_worker, a_b_share_shifted);
    }

    fn receive_share(&self, stage: usize) -> (T, T) {
        loop {
            let stage_shares = self.worker.stage_shares.lock().unwrap();
            if stage_shares.len() != stage + 1 {
                drop(stage_shares);
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            if !stage_shares[stage].contains_key(&self.worker.index) {
                drop(stage_shares);
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            let tuple = stage_shares[stage].get(&self.worker.index).unwrap();
            return (tuple.0.clone(), tuple.1.clone());
        }
    }
}
fn main() {
    let x = Bls381K12Scalar::from_usize(100);

    let expected = x * x * x
        + Bls381K12Scalar::from_usize(5) * x * x
        + Bls381K12Scalar::from_usize(3) * x
        + Bls381K12Scalar::from_usize(2);

    // zkpd for x^3 + 5x^2 + 3x + 2

    let w1 = ExampleWorker::<Bls381K12Scalar> {
        _marker: std::marker::PhantomData,
        index: 1,
        peer_workers: Mutex::new(vec![]),
        stage_shares: Mutex::new(vec![]),
    };
    let w2 = ExampleWorker::<Bls381K12Scalar> {
        _marker: std::marker::PhantomData,
        index: 2,
        peer_workers: Mutex::new(vec![]),
        stage_shares: Mutex::new(vec![]),
    };
    let c1 = ExampleWorkerClient::<Bls381K12Scalar> {
        _marker: std::marker::PhantomData,
        worker: Arc::new(w1),
    };
    let c2 = ExampleWorkerClient::<Bls381K12Scalar> {
        _marker: std::marker::PhantomData,
        worker: Arc::new(w2),
    };

    let worker_clients: Vec<Arc<dyn WorkerClient<Bls381K12Scalar>>> =
        vec![Arc::new(c1), Arc::new(c2)];

    let d = ExampleDelegator::<Bls381K12Scalar>::new(worker_clients);

    let result = d.delegate(vec![x]);

    println!("result:{:?}, expected:{:?}", result, expected);
    assert!(result.len() == 1 && result[0] == expected);
}
