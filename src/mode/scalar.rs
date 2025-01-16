use std::sync::Arc;

use crate::FiniteField;

pub trait Delegator<T: FiniteField> {
    fn new(workers: Vec<Arc<dyn WorkerClient<T>>>) -> Self;
    fn delegate(&self, inputs: Vec<T>) -> Vec<T>;
}

pub trait Base<T: FiniteField> {
    fn index(&self) -> usize;
    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T>;
}

pub trait Worker<T: FiniteField>: Base<T> {
    fn broadcast(&self, _a_b_share_shifted: (T, T), _stage: usize);
    fn wait_for_broadcast(&self, _stage: usize) -> (T, T);
    fn multiply(&self, stage: usize, a_share: T, b_share: T, r: &(T, T, T)) -> T {
        let (alpha, beta, gamma) = r;
        let a_share_shifted = a_share.sub(alpha);
        let b_share_shifted = b_share.sub(beta);
        self.broadcast((a_share_shifted, b_share_shifted), stage);
        let (recovered_a_share_shifted, recovered_b_share_shifted) = self.wait_for_broadcast(stage);

        gamma
            .clone()
            .add(&recovered_a_share_shifted.clone().mul(&beta))
            .add(&recovered_b_share_shifted.clone().mul(&alpha))
            .add(&recovered_a_share_shifted.mul(&recovered_b_share_shifted))
    }
}

// Send + Sync is required to make Vec<Arc<dyn WorkerClient<T>>> implement rayon::ParallelIterator
pub trait WorkerClient<T: FiniteField>: Base<T> + Send + Sync {
    fn set_peer_workers(&self, peer_workers: Vec<Arc<dyn WorkerClient<T>>>);
    fn send_share(&self, from_worker: usize, a_b_share_shifted: (T, T), stage: usize);
    fn receive_share(&self, stage: usize) -> (T, T);
}
