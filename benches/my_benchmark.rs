use criterion::{black_box, criterion_group, criterion_main, Criterion};

use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::secret_sharing::SecretSharing;
use zkpd::{FiniteField, SecretSharing as SecretSharingTrait};

fn share_recover(secret: &Bls381K12Scalar) {
    let n = 5;
    let shares = SecretSharing::share(secret.clone(), n, n);
    assert_eq!(
        *secret,
        SecretSharing::recover(shares.clone(), vec![1, 2, 3, 4, 5], n, n)
    );
}

fn criterion_benchmark(c: &mut Criterion) {
    let secret = Bls381K12Scalar::random();
    c.bench_function("share_recover", |b| {
        b.iter(|| share_recover(black_box(&secret)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
