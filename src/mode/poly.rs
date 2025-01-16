use std::sync::Arc;

use crate::util::{evaluations, interpolate, is_power_of_two};
use crate::FiniteField;

pub trait Delegator<T: FiniteField> {
    fn new(workers: Vec<Arc<dyn WorkerClient<T>>>) -> Self;
    fn delegate(&self, poly1: Vec<T>, poly2: Vec<T>) -> Vec<T>;
}

pub trait Base<T: FiniteField> {
    fn index(&self) -> usize;
    fn work(
        &self,
        beaver_triple_shares: Vec<(T, T, T)>,
        poly1_shares: Vec<T>,
        poly2_shares: Vec<T>,
    ) -> Vec<T>;
}

pub trait Worker<T: FiniteField>: Base<T> {
    fn multiply_poly(
        &self,
        stage: usize,
        a_poly_shares: Vec<T>,
        b_poly_shares: Vec<T>,
        rs: &[(T, T, T)],
    ) -> Vec<T> {
        let use_fft =
            b_poly_shares.len() == b_poly_shares.len() && is_power_of_two(a_poly_shares.len());
        let n = if use_fft {
            a_poly_shares.len() + b_poly_shares.len()
        } else {
            a_poly_shares.len() + b_poly_shares.len() - 1
        };
        let a_evaluations = evaluations(&a_poly_shares, n);
        let b_evaluations = evaluations(&b_poly_shares, n);
        let mut to_broadcast = vec![];
        for i in 0..n {
            let a_share = a_evaluations[i].clone();
            let b_share = b_evaluations[i].clone();
            let r = &rs[i];
            let (alpha, beta, _) = r;
            let a_share_shifted = a_share.sub(alpha);
            let b_share_shifted = b_share.sub(beta);
            to_broadcast.push((a_share_shifted, b_share_shifted));
        }
        self.broadcast_poly(to_broadcast, stage);
        let recovered_shares = self.wait_for_broadcast_poly(stage);
        let mut products = vec![];
        for (i, (recovered_a_share_shifted, recovered_b_share_shifted)) in
            recovered_shares.iter().enumerate()
        {
            let r = &rs[i];
            let (alpha, beta, gamma) = r;
            products.push(
                gamma
                    .clone()
                    .add(&recovered_a_share_shifted.clone().mul(&beta))
                    .add(&recovered_b_share_shifted.clone().mul(&alpha))
                    .add(
                        &recovered_a_share_shifted
                            .clone()
                            .mul(&recovered_b_share_shifted),
                    ),
            );
        }
        interpolate(&products, n)
    }

    fn broadcast_poly(&self, _a_b_share_shifted: Vec<(T, T)>, _stage: usize);
    fn wait_for_broadcast_poly(&self, _stage: usize) -> Vec<(T, T)>;
}

// Send + Sync is required to make Vec<Arc<dyn WorkerClient<T>>> implement rayon::ParallelIterator
pub trait WorkerClient<T: FiniteField>: Base<T> + Send + Sync {
    fn set_peer_workers(&self, peer_workers: Vec<Arc<dyn WorkerClient<T>>>);
    fn send_share(&self, from_worker: usize, a_b_share_shifted: Vec<(T, T)>, stage: usize);
    fn receive_share(&self, stage: usize) -> Vec<(T, T)>;
}
