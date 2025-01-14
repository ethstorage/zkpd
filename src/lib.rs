pub mod beaver_triple_generatoor;
pub mod ff;
pub mod mode;
pub mod secret_sharing;
pub mod util;

pub trait FiniteField: Send + Sync + PartialEq + Clone {
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
    fn minus(self) -> Self;
}

pub trait SecretSharing<T: FiniteField> {
    fn share(secret: T, n: usize, t: usize) -> Vec<T>;
    fn recover(shares: Vec<T>, indexes: Vec<usize>, n: usize, t: usize) -> T;
}

pub trait BeaverTripleGeneratoor<T> {
    fn generate() -> (T, T, T);
}
