use criterion::{black_box, criterion_group, criterion_main, Criterion};

use std::ops::{Add, Sub};
use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::secret_sharing::SecretSharing;
use zkpd::{FiniteField, SecretSharing as SecretSharingTrait};

fn share_recover(secret: &Bls381K12Scalar) {
    let n = 3;
    let shares = SecretSharing::share(secret.clone(), n, n);
    assert_eq!(
        *secret,
        SecretSharing::recover(shares, (1..=n).map(|i| i).collect(), n, n)
    );
}

fn naive_share_recover(secret: &Bls381K12Scalar) {
    let n = 3;
    let shares = naive_share(secret, n);
    assert_eq!(*secret, naive_recover(shares));
}

fn naive_share(secret: &Bls381K12Scalar, n: usize) -> Vec<Bls381K12Scalar> {
    let mut result = Vec::with_capacity(n);
    let mut sum = Bls381K12Scalar::zero();
    for _i in 0..n - 1 {
        let r = Bls381K12Scalar::random();
        sum = sum.add(&r);
        result.push(r);
    }
    result.push(secret.sub(&sum));
    result
}

fn naive_recover(shares: Vec<Bls381K12Scalar>) -> Bls381K12Scalar {
    let mut sum = Bls381K12Scalar::zero();
    for item in shares {
        sum = sum.add(&item);
    }
    sum
}

fn criterion_benchmark(c: &mut Criterion) {
    let secret = Bls381K12Scalar::random();
    c.bench_function("share_recover", |b| {
        b.iter(|| share_recover(black_box(&secret)))
    });
    c.bench_function("naive_share_recover", |b| {
        b.iter(|| naive_share_recover(black_box(&secret)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
