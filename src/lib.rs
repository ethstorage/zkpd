use std::sync::Arc;

extern crate ff as ff2;

pub mod beaver_triple_generatoor;
pub mod ff;
pub mod secret_sharing;

pub trait FiniteField {
    fn random() -> Self;
    fn zero() -> Self;
    fn one() -> Self;
    fn from_usize(n: usize) -> Self;
    /* returns `partial * point + coef`, used by Horner's method. */
    fn horner_fold(partial: &Self, coef: &Self, point: &Self) -> Self;
    fn mul(self, other: &Self) -> Self;
    fn sub(self, other: &Self) -> Self;
    fn add(self, other: &Self) -> Self;
    fn div(self, other: &Self) -> Self;
    fn clone(&self) -> Self;
}

pub trait SecretSharing<T: FiniteField> {
    fn share(secret: T, n: usize, t: usize) -> Vec<T>;
    fn recover(shares: Vec<T>, indexes: Vec<usize>, n: usize, t: usize) -> T;
}

pub trait BeaverTripleGeneratoor<T> {
    fn generate() -> (T, T, T);
}

pub trait Delegator<T: FiniteField> {
    fn new(workers: Vec<Arc<Box<dyn Worker<T>>>>) -> Self;
    fn delegate(&self, inputs: Vec<T>) -> Vec<T>;
}
pub trait Worker<T: FiniteField> {
    fn index(&self) -> usize;
    fn set_peer_workers(&self, peer_workers: Vec<Arc<Box<dyn Worker<T>>>>);
    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T>;
    fn broadcast(&self, a_share_shifted: T, b_share_shifted: T, stage: usize);
    fn wait_for_broadcast(&self) -> (T, T);
    fn multiply(&self, stage: usize, a_share: T, b_share: T, r: (T, T, T)) -> T {
        let (alpha, beta, gamma) = r;
        let a_share_shifted = a_share.sub(&alpha);
        let b_share_shifted = b_share.sub(&beta);
        self.broadcast(a_share_shifted, b_share_shifted, stage);
        let (sum_a_share_shifted, sum_b_share_shifted) = self.wait_for_broadcast();

        gamma.add(&sum_a_share_shifted.clone().mul(&beta)).add(&sum_b_share_shifted.clone().mul(&alpha)).add(&sum_a_share_shifted.mul(&sum_b_share_shifted))
    }
}
