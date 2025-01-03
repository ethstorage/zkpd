use ff2::*;
use rand::rngs::OsRng;

/// how does Bls381K12Scalar implement Send and Sync?
/// FYI: https://doc.rust-lang.org/nomicon/send-and-sync.html
/// TLDR: most types are Send and Sync, as long as they don't wrap any pointer types.
///
/// The BLS12-381 scalar field.
#[derive(PrimeField)]
#[PrimeFieldModulus = "52435875175126190479447740508185965837690552500527637822603658699938581184513"]
#[PrimeFieldGenerator = "7"]
#[PrimeFieldReprEndianness = "little"]
pub struct Bls381K12Scalar([u64; 4]);

impl crate::FiniteField for Bls381K12Scalar {
    fn random() -> Self {
        Field::random(OsRng)
    }

    fn zero() -> Self {
        Bls381K12Scalar::ZERO
    }

    fn one() -> Self {
        Bls381K12Scalar::ONE
    }

    fn from_usize(n: usize) -> Self {
        Self::from(n as u64)
    }

    fn horner_fold(partial: &Self, coef: &Self, point: &Self) -> Self {
        partial.mul(point).add(coef)
    }

    fn mul(self, other: &Self) -> Self {
        self * other
    }

    fn sub(self, other: &Self) -> Self {
        self - other
    }

    fn add(self, other: &Self) -> Self {
        self + other
    }

    fn div(self, other: &Self) -> Self {
        self * other.invert().unwrap()
    }

    fn clone(&self) -> Self {
        *self
    }
}
