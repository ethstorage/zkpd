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
    fn delegate(&self, inputs: Vec<T>, workers: Vec<Arc<impl Worker<T>>>) -> Vec<T>;
}
pub trait Worker<T: FiniteField> {
    fn set_peer_workers(&self, peer_workers: Vec<Arc<Self>>);
    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T>;
    fn broadcast(&self, intermediate_shares: Vec<T>, stage: usize);
    fn wait_for_broadcast(&self, stage: usize);
}
