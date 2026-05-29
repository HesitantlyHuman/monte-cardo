#[pyo3::pymodule]
mod monte_cardo {
    use pyo3::prelude::*;

    #[pyfunction]
    fn example(a: usize, b: usize) -> usize {
        return a + b;
    }
}
