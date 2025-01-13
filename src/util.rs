use core::panic;

use crate::FiniteField;

pub fn evaluations<T: FiniteField>(poly: &[T], n: usize) -> Vec<T> {
    if is_power_of_two(n) {
        return evaluations_fft(poly, n);
    }
    let mut evals = vec![];
    for i in 0..n {
        let x = T::from_usize(i + 1);
        let mut eval = poly[poly.len() - 1].clone();
        for j in (0..poly.len() - 1).rev() {
            eval = T::horner_fold(&eval, &poly[j], &x);
        }
        evals.push(eval);
    }
    evals
}

fn evaluations_fft<T: FiniteField>(poly: &[T], n: usize) -> Vec<T> {
    panic!("fft evaluation to be implemented");
    // let mut evals = vec![T::zero(); n];
    // let mut poly = poly.to_vec();
    // poly.resize(n, T::zero());
    // let omega = T::get_root_of_unity(n);
    // let mut omega_power = T::one();
    // for i in 0..n {
    //     let mut eval = T::zero();
    //     for j in 0..n {
    //         eval = eval.add(&poly[j].clone().mul(&omega_power.pow(&j)));
    //     }
    //     evals[i] = eval;
    //     omega_power = omega_power.mul(&omega);
    // }
    // evals
}

pub fn interpolate_eval<T: FiniteField, Func: Fn(usize) -> T>(
    interpolate_points: Func,
    values: &[T],
    eval_point: &T,
) -> T {
    let mut result = T::zero();
    for i in 0..values.len() {
        let original_index_i = interpolate_points(i);

        let mut num = T::one();
        let mut den = T::one();
        for j in 0..values.len() {
            if i == j {
                continue;
            }
            let original_index_j = interpolate_points(j);
            assert!(
                original_index_i != original_index_j,
                "index should not be the same"
            );
            num = num.mul(&eval_point.clone().sub(&original_index_j));
            den = den.mul(&original_index_i.clone().sub(&original_index_j));
        }
        result = result.add(&values[i].clone().mul(&num.div(&den)));
    }
    result
}

pub fn naive_mul<T: FiniteField>(poly1: &[T], poly2: &[T]) -> Vec<T> {
    let n = poly1.len() + poly2.len() - 1;
    let mut result = vec![T::zero(); n];
    for (i, coeff_i) in poly1.iter().enumerate() {
        for (j, coeff_j) in poly2.iter().enumerate() {
            result[i + j] = result[i + j].clone().add(&coeff_i.clone().mul(coeff_j));
        }
    }
    result
}

pub fn naive_add<T: FiniteField>(poly1: &[T], poly2: &[T]) -> Vec<T> {
    let n = poly1.len().max(poly2.len());
    let mut result = vec![T::zero(); n];
    for (i, coeff) in poly1.iter().enumerate() {
        result[i] = result[i].clone().add(coeff);
    }
    for (i, coeff) in poly2.iter().enumerate() {
        result[i] = result[i].clone().add(coeff);
    }
    result
}

pub fn scalar_mul<T: FiniteField>(poly: &[T], scalar: &T) -> Vec<T> {
    poly.iter().map(|coeff| coeff.clone().mul(scalar)).collect()
}

pub fn scalar_div<T: FiniteField>(poly: &[T], scalar: &T) -> Vec<T> {
    poly.iter().map(|coeff| coeff.clone().div(scalar)).collect()
}

pub fn interpolate<T: FiniteField>(evaluations: &[T], n: usize) -> Vec<T> {
    assert!(evaluations.len() == n, "size mismatch");
    if is_power_of_two(n) {
        panic!("fft interpolate to be implemented");
    }

    let mut poly = vec![];
    for i in 0..n {
        let mut term = vec![evaluations[i].clone()];
        for j in 0..n {
            if j != i {
                let den = T::from_usize(i + 1).sub(&T::from_usize(j + 1));
                term = naive_mul(&term, &[T::from_usize(j + 1).minus(), T::one()]);
                term = scalar_div(&term, &den);
            }
        }
        poly = naive_add(&poly, &term);
    }
    poly
}

pub fn is_power_of_two(n: usize) -> bool {
    n > 0 && (n & (n - 1)) == 0
}
