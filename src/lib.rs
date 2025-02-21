pub mod beaver_triple_generatoor;
pub mod ff;
pub mod mode;
pub mod p2p;
pub mod secret_sharing;
pub mod util;
use std::ops::{Add, Div, Mul, Neg, Sub};

pub trait FiniteField:
    Send
    + Sync
    + PartialEq
    + Clone
    + for<'a> Add<&'a Self, Output = Self>
    + for<'a> Sub<&'a Self, Output = Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + for<'a> Div<&'a Self, Output = Self>
    + for<'a> Neg<Output = Self>
{
    fn random() -> Self;
    fn zero() -> Self;
    fn one() -> Self;
    fn from_usize(n: usize) -> Self;
    /* returns `partial * point + coef`, used by Horner's method. */
    fn horner_fold(partial: &Self, coef: &Self, point: &Self) -> Self;
}

pub trait SecretSharing<T: FiniteField> {
    fn share(secret: T, n: usize, t: usize) -> Vec<T>;
    fn recover(shares: Vec<T>, indexes: Vec<usize>, n: usize, t: usize) -> T;
}

pub trait BeaverTripleGeneratoor<T> {
    fn generate() -> (T, T, T);
}
