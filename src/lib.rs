use std::sync::Arc;

extern crate ff as ff2;

pub mod beaver_triple_generatoor;
pub mod ff;
pub mod secret_sharing;
mod util;

pub trait FiniteField: Send + Sync {
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
    fn new(workers: Vec<Arc<dyn WorkerClient<T>>>) -> Self;
    fn delegate(&self, inputs: Vec<T>) -> Vec<T>;
}

pub trait Base<T: FiniteField> {
    fn index(&self) -> usize;
    fn work(&self, beaver_triple_shares: Vec<(T, T, T)>, input_shares: Vec<T>) -> Vec<T>;
}
pub trait Worker<T: FiniteField>: Base<T> {
    fn broadcast(&self, a_b_share_shifted: (T, T), stage: usize);
    fn wait_for_broadcast(&self, stage: usize) -> (T, T);
    fn multiply(&self, stage: usize, a_share: T, b_share: T, r: &(T, T, T)) -> T {
        let (alpha, beta, gamma) = r;
        let a_share_shifted = a_share.sub(alpha);
        let b_share_shifted = b_share.sub(beta);
        self.broadcast((a_share_shifted, b_share_shifted), stage);
        let (sum_a_share_shifted, sum_b_share_shifted) = self.wait_for_broadcast(stage);

        gamma
            .clone()
            .add(&sum_a_share_shifted.clone().mul(&beta))
            .add(&sum_b_share_shifted.clone().mul(&alpha))
            .add(&sum_a_share_shifted.mul(&sum_b_share_shifted))
    }
    // fn multiply_poly(
    //     &self,
    //     stage: usize,
    //     a_poly_shares: Vec<T>,
    //     b_poly_shares: Vec<T>,
    //     rs: &[(T, T, T)],
    // ) -> Vec<T> {
    //     let use_fft =
    //         b_poly_shares.len() == b_poly_shares.len() && is_power_of_two(a_poly_shares.len());
    //     let n = if use_fft {
    //         a_poly_shares.len() + b_poly_shares.len()
    //     } else {
    //         a_poly_shares.len() + b_poly_shares.len() - 1
    //     };
    //     let a_evaluations = Self::evaluations(a_poly_shares, n);
    //     let b_evaluations = Self::evaluations(b_poly_shares, n);
    //     let mut to_broadcast = vec![];
    //     for i in 0..n {
    //         let a_share = a_evaluations[i].clone();
    //         let b_share = b_evaluations[i].clone();
    //         let r = &rs[i];
    //         let (alpha, beta, _) = r;
    //         let a_share_shifted = a_share.sub(alpha);
    //         let b_share_shifted = b_share.sub(beta);
    //         to_broadcast.push((a_share_shifted, b_share_shifted));
    //     }
    //     self.broadcast_poly(to_broadcast, stage);
    //     let sums = self.wait_for_broadcast_poly(stage);
    //     let mut products = vec![];
    //     for (i, (sum_a_share_shifted, sum_b_share_shifted)) in sums.iter().enumerate() {
    //         let r = &rs[i];
    //         let (alpha, beta, gamma) = r;
    //         products.push(
    //             gamma
    //                 .clone()
    //                 .add(&sum_a_share_shifted.clone().mul(&beta))
    //                 .add(&sum_b_share_shifted.clone().mul(&alpha))
    //                 .add(&sum_a_share_shifted.clone().mul(&sum_b_share_shifted)),
    //         );
    //     }
    //     Self::interpolate(products, n)
    // }

    // fn broadcast_poly(&self, a_b_share_shifted: Vec<(T, T)>, stage: usize);
    // fn wait_for_broadcast_poly(&self, stage: usize) -> Vec<(T, T)>;
    // fn evaluations(poly_shares: Vec<T>, n: usize) -> Vec<T> {
    //     if is_power_of_two(n) {
    //         panic!("To be implemented");
    //     } else {
    //         let mut evaluations = vec![];
    //         for i in 0..n {
    //             let mut sum = T::zero();
    //             for (j, share) in poly_shares.iter().enumerate() {
    //                 sum = T::horner_fold(&sum, share, &T::from_usize(j));
    //             }
    //             evaluations.push(sum);
    //         }
    //         return evaluations;
    //     }
    // }
    // fn interpolate(evaluations: Vec<T>, n: usize) -> Vec<T> {
    //     if is_power_of_two(n) {
    //         panic!("To be implemented");
    //     } else {
    //         let mut poly_shares = vec![];
    //         for i in 0..n {
    //             let mut sum = T::zero();
    //             for (j, evaluation) in evaluations.iter().enumerate() {
    //                 let mut product = evaluation.clone();
    //                 for k in 0..n {
    //                     if k != j {
    //                         product = product.mul(&T::from_usize(i).sub(&T::from_usize(k)));
    //                         product = product.div(&T::from_usize(j).sub(&T::from_usize(k)));
    //                     }
    //                 }
    //                 sum = sum.add(&product);
    //             }
    //             poly_shares.push(sum);
    //         }
    //         return poly_shares;
    //     }
    // }
}

// Send + Sync is required to make Vec<Arc<dyn WorkerClient<T>>> implement rayon::ParallelIterator
pub trait WorkerClient<T: FiniteField>: Base<T> + Send + Sync {
    fn set_peer_workers(&self, peer_workers: Vec<Arc<dyn WorkerClient<T>>>);
    fn send_share(&self, from_worker: usize, a_b_share_shifted: (T, T), stage: usize);
    fn receive_share(&self, stage: usize) -> (T, T);
}
