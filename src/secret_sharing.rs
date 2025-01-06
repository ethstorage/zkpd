use crate::util::evaluations;
use std::marker::PhantomData;

pub struct SecretSharing<T: crate::FiniteField> {
    _marker: PhantomData<T>,
}

impl<T: crate::FiniteField> crate::SecretSharing<T> for SecretSharing<T> {
    fn share(secret: T, n: usize, t: usize) -> Vec<T> {
        let mut poly = Vec::new();
        poly.push(secret);
        for _ in 1..t {
            poly.push(T::random());
        }
        evaluations(&poly, n)
    }

    fn recover(shares: Vec<T>, indexes: Vec<usize>, n: usize, t: usize) -> T {
        assert!(
            shares.len() == indexes.len() && shares.len() == t,
            "size mismatch"
        );
        let mut secret = T::zero();
        for i in 0..shares.len() {
            let original_index_i = indexes[i];
            assert!(
                original_index_i <= n,
                "index out of bound, original_index_i:{}, n:{}",
                original_index_i,
                n
            );
            let mut num = T::one();
            let mut den = T::one();
            for j in 0..shares.len() {
                if i == j {
                    continue;
                }
                let original_index_j = indexes[j];
                assert!(original_index_j <= n, "index out of bound");
                assert!(
                    original_index_i != original_index_j,
                    "index should not be the same"
                );
                num = num.mul(&T::zero().sub(&T::from_usize(original_index_j)));
                den =
                    den.mul(&T::from_usize(original_index_i).sub(&T::from_usize(original_index_j)));
            }
            secret = secret.add(&shares[i].clone().mul(&num.div(&den)));
        }
        secret
    }
}

#[test]
fn share_and_recover_works() {
    use crate::ff::bls12_381::Bls381K12Scalar;
    use crate::{FiniteField, SecretSharing as SecretSharingTrait};
    let secret = Bls381K12Scalar::random();
    let shares = SecretSharing::share(FiniteField::clone(&secret), 5, 5);
    assert_eq!(
        secret,
        SecretSharing::recover(shares.clone(), vec![1, 2, 3, 4, 5], 5, 5)
    );
    assert_ne!(
        secret,
        SecretSharing::recover(shares, vec![1, 2, 4, 5, 3], 5, 5)
    );
}
