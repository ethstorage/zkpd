use std::marker::PhantomData;
pub struct BeaverTripleGeneratoor<T: crate::FiniteField> {
    _marker: PhantomData<T>,
}

impl<T: crate::FiniteField> crate::BeaverTripleGeneratoor<T> for BeaverTripleGeneratoor<T> {
    fn generate() -> (T, T, T) {
        let a = T::random();
        let b = T::random();
        let c = a.clone().mul(&b);
        (a, b, c)
    }
}
