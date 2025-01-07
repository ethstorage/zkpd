use crate::FiniteField;

pub fn evaluations<T: FiniteField>(poly: &[T], n: usize) -> Vec<T> {
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

pub fn is_power_of_two(n: usize) -> bool {
    n > 0 && (n & (n - 1)) == 0
}
