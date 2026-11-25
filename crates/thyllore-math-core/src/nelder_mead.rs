#[derive(Clone, Debug)]
pub struct NelderMeadParams {
    pub initial_step: f32,
    pub max_iterations: usize,
    pub tolerance: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NelderMeadResult {
    pub point: Vec<f32>,
    pub value: f32,
    pub iterations: usize,
}

const REFLECTION: f32 = 1.0;
const EXPANSION: f32 = 2.0;
const CONTRACTION: f32 = 0.5;
const SHRINK: f32 = 0.5;

fn simplex_values(n: usize, start: &[f32], step: f32) -> Vec<Vec<f32>> {
    let mut simplex = vec![start.to_vec()];
    for i in 0..n {
        let mut point = start.to_vec();
        point[i] += step;
        simplex.push(point);
    }
    simplex
}

fn sort_simplex(simplex: &mut [Vec<f32>], values: &mut [f32]) {
    let n = simplex.len();
    for v in values.iter_mut() {
        if v.is_nan() {
            *v = f32::MAX;
        }
    }
    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|a, b| values[*a].total_cmp(&values[*b]));
    let new_simplex: Vec<Vec<f32>> = indices.iter().map(|&i| simplex[i].clone()).collect();
    let new_values: Vec<f32> = indices.iter().map(|&i| values[i]).collect();
    for i in 0..n {
        simplex[i] = new_simplex[i].clone();
        values[i] = new_values[i];
    }
}

fn centroid(simplex: &[Vec<f32>], n: usize) -> Vec<f32> {
    let mut c = vec![0.0f32; n];
    for i in 0..n {
        for v in &simplex[..n] {
            c[i] += v[i];
        }
        c[i] /= n as f32;
    }
    c
}

fn reflect(centroid: &[f32], worst: &[f32]) -> Vec<f32> {
    centroid
        .iter()
        .zip(worst.iter())
        .map(|(&c, &w)| c + REFLECTION * (c - w))
        .collect()
}

fn expand(centroid: &[f32], reflected: &[f32]) -> Vec<f32> {
    centroid
        .iter()
        .zip(reflected.iter())
        .map(|(&c, &r)| c + EXPANSION * (r - c))
        .collect()
}

fn contract(centroid: &[f32], point: &[f32]) -> Vec<f32> {
    centroid
        .iter()
        .zip(point.iter())
        .map(|(&c, &p)| c + CONTRACTION * (p - c))
        .collect()
}

fn shrink_simplex(simplex: &mut [Vec<f32>], best: &[f32]) {
    let n = simplex.len();
    for i in 1..n {
        for d in 0..simplex[i].len() {
            simplex[i][d] = best[d] + SHRINK * (simplex[i][d] - best[d]);
        }
    }
}

fn handle_contraction(
    simplex: &mut [Vec<f32>],
    values: &mut [f32],
    objective: &mut impl FnMut(&[f32]) -> f32,
    c: &[f32],
    r: &[f32],
    f_r: f32,
    n: usize,
) {
    let w = values[n];
    if f_r < w {
        let co = contract(c, &r);
        let f_co = objective(&co);
        if f_co <= f_r {
            simplex[n] = co;
            values[n] = f_co;
        } else {
            shrink_and_recompute(simplex, values, objective, n);
        }
    } else {
        let co = contract(c, &simplex[n]);
        let f_co = objective(&co);
        if f_co < w {
            simplex[n] = co;
            values[n] = f_co;
        } else {
            shrink_and_recompute(simplex, values, objective, n);
        }
    }
}

fn shrink_and_recompute(
    simplex: &mut [Vec<f32>],
    values: &mut [f32],
    objective: &mut impl FnMut(&[f32]) -> f32,
    n: usize,
) {
    let best = simplex[0].clone();
    shrink_simplex(simplex, &best);
    for i in 1..=n {
        values[i] = objective(&simplex[i]);
    }
}

fn step_simplex(
    simplex: &mut [Vec<f32>],
    values: &mut [f32],
    objective: &mut impl FnMut(&[f32]) -> f32,
) {
    let n = simplex.len() - 1;
    let c = centroid(simplex, n);
    let r = reflect(&c, &simplex[n]);
    let f_r = objective(&r);

    if f_r < values[0] {
        let e = expand(&c, &r);
        let f_e = objective(&e);
        if f_e < f_r {
            simplex[n] = e;
            values[n] = f_e;
        } else {
            simplex[n] = r;
            values[n] = f_r;
        }
    } else if f_r < values[n - 1] {
        simplex[n] = r;
        values[n] = f_r;
    } else {
        handle_contraction(simplex, values, objective, &c, &r, f_r, n);
    }
}

pub fn minimize_nelder_mead(
    mut objective: impl FnMut(&[f32]) -> f32,
    start: &[f32],
    params: &NelderMeadParams,
) -> NelderMeadResult {
    let n = start.len();
    let mut simplex = simplex_values(n, start, params.initial_step);
    let mut values: Vec<f32> = simplex.iter().map(|p| objective(p)).collect();

    sort_simplex(&mut simplex, &mut values);

    let mut iterations = 0;
    while iterations < params.max_iterations {
        if (values[n] - values[0]).abs() <= params.tolerance {
            break;
        }

        step_simplex(&mut simplex, &mut values, &mut objective);
        sort_simplex(&mut simplex, &mut values);
        iterations += 1;
    }

    NelderMeadResult {
        point: simplex[0].clone(),
        value: values[0],
        iterations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic() {
        let objective = |x: &[f32]| -> f32 { (x[0] - 1.0).powi(2) + (x[1] + 2.0).powi(2) };
        let start = [0.0, 0.0];
        let params = NelderMeadParams {
            initial_step: 1.0,
            max_iterations: 1000,
            tolerance: 1e-6,
        };
        let result = minimize_nelder_mead(objective, &start, &params);
        assert!((result.point[0] - 1.0).abs() < 1e-3);
        assert!((result.point[1] + 2.0).abs() < 1e-3);
    }

    #[test]
    fn test_rosenbrock() {
        let objective =
            |x: &[f32]| -> f32 { (1.0 - x[0]).powi(2) + 100.0 * (x[1] - x[0] * x[0]).powi(2) };
        let start = [-1.2, 1.0];
        let params = NelderMeadParams {
            initial_step: 0.5,
            max_iterations: 2000,
            tolerance: 1e-6,
        };
        let result = minimize_nelder_mead(objective, &start, &params);
        assert!((result.point[0] - 1.0).abs() < 1e-2);
        assert!((result.point[1] - 1.0).abs() < 1e-2);
    }

    #[test]
    fn test_zero_iterations() {
        let objective = |x: &[f32]| -> f32 { x[0] * x[0] + x[1] * x[1] };
        let start = [3.0, 4.0];
        let params = NelderMeadParams {
            initial_step: 1.0,
            max_iterations: 0,
            tolerance: 1e-6,
        };
        let result = minimize_nelder_mead(objective, &start, &params);
        assert_eq!(result.point, vec![3.0, 4.0]);
        assert_eq!(result.iterations, 0);
    }
}
