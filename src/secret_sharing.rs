use crate::util::{evaluations, interpolate_eval};
use std::marker::PhantomData;

pub struct SecretSharing<T: crate::FiniteField> {
    _marker: PhantomData<T>,
}

impl<T: crate::FiniteField> crate::SecretSharing<T> for SecretSharing<T> {
    fn share(secret: T, n: usize, t: usize) -> Vec<T> {
        let mut poly = Vec::with_capacity(t);
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
        interpolate_eval(
            |i| {
                let original_index_i = indexes[i];
                assert!(
                    original_index_i <= n,
                    "index out of bound, original_index_i:{}, n:{}, i:{}",
                    original_index_i,
                    n,
                    i
                );
                T::from_usize(original_index_i)
            },
            &shares,
            &T::zero(),
        )
    }
}

#[test]
fn share_and_recover_works() {
    use crate::ff::bls12_381::Bls381K12Scalar;
    use crate::{FiniteField, SecretSharing as SecretSharingTrait};
    let secret = Bls381K12Scalar::random();
    let shares = SecretSharing::share(secret.clone(), 5, 5);
    assert_eq!(
        secret,
        SecretSharing::recover(shares.clone(), vec![1, 2, 3, 4, 5], 5, 5)
    );
    assert_ne!(
        secret,
        SecretSharing::recover(shares, vec![1, 2, 4, 5, 3], 5, 5)
    );
}
