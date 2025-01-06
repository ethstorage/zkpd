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

pub fn is_power_of_two(n: usize) -> bool {
    n > 0 && (n & (n - 1)) == 0
}
