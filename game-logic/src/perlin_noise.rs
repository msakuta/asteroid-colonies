//! Perlin noise implementation from Wikipedia https://en.wikipedia.org/wiki/Perlin_noise

use crate::Xor128;

pub(crate) fn perlin_noise_pixel(x: f64, y: f64, bit: u32, terms: &[[f64; 6]]) -> f64 {
    let mut sum = 0.;
    let [mut maxv, mut f] = [0., 1.];
    let persistence = 0.75;
    for i in (0..bit).rev() {
        let cell = 1 << i;
        let fcell = cell as f64;
        let dx = x / fcell;
        let dy = y / fcell;
        let x0 = dx.floor();
        let x1 = x0 + 1.;
        let y0 = dy.floor();
        let y1 = y0 + 1.;
        let a00 = noise_pixel(x0, y0, dx, dy, &terms[i as usize]);
        let a01 = noise_pixel(x0, y1, dx, dy, &terms[i as usize]);
        let a10 = noise_pixel(x1, y0, dx, dy, &terms[i as usize]);
        let a11 = noise_pixel(x1, y1, dx, dy, &terms[i as usize]);
        let fx = dx - x0;
        let fy = dy - y0;
        sum += ((a00 * (1. - fx) + a10 * fx) * (1. - fy) + (a01 * (1. - fx) + a11 * fx) * fy) * f;
        maxv += f;
        f *= persistence;
    }
    sum / maxv
}

pub(crate) fn gen_terms(rng: &mut Xor128, bit: u32) -> Vec<[f64; 6]> {
    (0..bit)
        .map(|_| {
            [
                10000. * rng.next(),
                10000. * rng.next(),
                std::f64::consts::PI * rng.next(),
                10000. * rng.next(),
                10000. * rng.next(),
                std::f64::consts::PI * rng.next(),
            ]
        })
        .collect()
}

fn random_gradient(x: f64, y: f64, terms: &[f64; 6]) -> [f64; 2] {
    let random = 2920.
        * (x * terms[0] + y * terms[1] + terms[2]).sin()
        * (x * terms[3] * y * terms[4] + terms[5]).cos();
    [random.cos(), random.sin()]
}

fn noise_pixel(ix: f64, iy: f64, x: f64, y: f64, terms: &[f64; 6]) -> f64 {
    // Get gradient from integer coordinates
    let gradient = random_gradient(ix, iy, terms);

    // Compute the distance vector
    let dx = x - ix;
    let dy = y - iy;

    // Compute the dot-product
    dx * gradient[0] + dy * gradient[1]
}
