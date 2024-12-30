use std::sync::Arc;

use zkpd::ff::bls12_381::Bls381K12Scalar;
use zkpd::secret_sharing::SecretSharing as SecretSharingImpl;
use zkpd::{
    beaver_triple_generatoor::BeaverTripleGeneratoor as BeaverTripleGeneratoorImpl,
    BeaverTripleGeneratoor, FiniteField, SecretSharing,
};
use zkpd::{Delegator, Worker};

struct ExampleDelegator<T: FiniteField> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: FiniteField> Delegator<T> for ExampleDelegator<T> {
    fn delegate(&self, inputs: Vec<T>, workers: Vec<Arc<impl Worker<T>>>) -> Vec<T> {
        assert!(inputs.len() == 1);
        let n = workers.len();
        vec![]
    }
}

struct ExampleWorker<T: FiniteField> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: FiniteField> Worker<T> for ExampleWorker<T> {
    fn set_peer_workers(&self, peer_workers: Vec<Arc<Self>>) {}
    
    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T> {
        vec![]
    }
    fn broadcast(&self, intermediate_shares: Vec<T>, stage: usize) {}
    fn wait_for_broadcast(&self, _stage: usize) {}
}

fn setup_random_shares(n: usize, t: usize) -> Vec<Bls381K12Scalar> {
    let mut shares = Vec::new();
    for _ in 0..3 {
        shares.push(Bls381K12Scalar::random());
    }
    shares
}

fn main() {
    let x = Bls381K12Scalar::from_usize(100);
    
    let expected = x * x * x
        + Bls381K12Scalar::from_usize(5) * x * x
        + Bls381K12Scalar::from_usize(3) * x
        + Bls381K12Scalar::from_usize(2);
    

    // zkpd for x^3 + 5x^2 + 3x + 2

    let d = ExampleDelegator::<Bls381K12Scalar> {
        _marker: std::marker::PhantomData,
    };

    let w1 = Arc::new( ExampleWorker::<Bls381K12Scalar> {
        _marker: std::marker::PhantomData,
    });
    let w2 = Arc::new(ExampleWorker::<Bls381K12Scalar> {
        _marker: std::marker::PhantomData,
    });
    let workers= vec![w1,w2];
    let result = d.delegate(vec![x], workers);

    assert!(result.len() == 1 && result[0] == expected);
}
