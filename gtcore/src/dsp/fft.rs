//! Minimal in-place radix-2 complex FFT. Sizes are small (512) and fixed, so
//! this is plain and allocation-free after construction; no external crate.

use std::f64::consts::PI;

pub struct Fft {
    n: usize,
    /// Bit-reversal permutation.
    rev: Vec<usize>,
    /// cos/sin of −2πk/n for k in 0..n/2.
    cos: Vec<f64>,
    sin: Vec<f64>,
}

impl Fft {
    /// `n` must be a power of two.
    pub fn new(n: usize) -> Self {
        assert!(n.is_power_of_two() && n >= 2, "FFT size must be a power of two");
        let bits = n.trailing_zeros();
        let rev = (0..n).map(|i| i.reverse_bits() >> (usize::BITS - bits)).collect();
        let cos = (0..n / 2).map(|k| (-2.0 * PI * k as f64 / n as f64).cos()).collect();
        let sin = (0..n / 2).map(|k| (-2.0 * PI * k as f64 / n as f64).sin()).collect();
        Fft { n, rev, cos, sin }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// Forward transform in place. `re` and `im` must both have length `n`.
    pub fn forward(&self, re: &mut [f64], im: &mut [f64]) {
        let n = self.n;
        assert!(re.len() == n && im.len() == n);
        for i in 0..n {
            let j = self.rev[i];
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= n {
            let half = size / 2;
            let step = n / size;
            for start in (0..n).step_by(size) {
                for k in 0..half {
                    let (wr, wi) = (self.cos[k * step], self.sin[k * step]);
                    let (a, b) = (start + k, start + k + half);
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            size *= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_naive_dft() {
        let n = 64;
        let x: Vec<f64> = (0..n).map(|i| ((i * 7 + 3) % 11) as f64 - 5.0).collect();
        let (mut re, mut im) = (x.clone(), vec![0.0; n]);
        Fft::new(n).forward(&mut re, &mut im);
        for k in 0..n {
            let (mut er, mut ei) = (0.0, 0.0);
            for (i, v) in x.iter().enumerate() {
                let a = -2.0 * PI * (k * i) as f64 / n as f64;
                er += v * a.cos();
                ei += v * a.sin();
            }
            assert!((re[k] - er).abs() < 1e-9 && (im[k] - ei).abs() < 1e-9, "bin {k}");
        }
    }
}
