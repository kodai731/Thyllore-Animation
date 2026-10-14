use crate::bounds::Aabb;
use crate::triangle_closest_point::closest_point_on_triangle;
use cgmath::{InnerSpace, Vector3};

#[derive(Clone, Debug)]
pub struct TriangleBvh {
    nodes: Vec<BvhNode>,
    triangle_order: Vec<u32>,
    positions: Vec<Vector3<f32>>,
    triangles: Vec<[u32; 3]>,
}

#[derive(Clone, Debug)]
struct BvhNode {
    bounds: Aabb,
    first: u32,
    count: u32,
    left_child: u32,
}

impl BvhNode {
    fn placeholder() -> Self {
        BvhNode {
            bounds: Aabb {
                min: Vector3::new(0.0, 0.0, 0.0),
                max: Vector3::new(0.0, 0.0, 0.0),
            },
            first: 0,
            count: 0,
            left_child: 0,
        }
    }
}

struct BvhBuilder<'a> {
    positions: &'a [Vector3<f32>],
    triangles: &'a [[u32; 3]],
    nodes: Vec<BvhNode>,
    triangle_order: Vec<u32>,
}

impl<'a> BvhBuilder<'a> {
    fn new(positions: &'a [Vector3<f32>], triangles: &'a [[u32; 3]]) -> Self {
        let n = triangles.len();
        let triangle_order: Vec<u32> = (0..n as u32).collect();
        let mut nodes = Vec::with_capacity(2 * n);
        nodes.push(BvhNode::placeholder());
        BvhBuilder {
            positions,
            triangles,
            nodes,
            triangle_order,
        }
    }

    fn build(mut self) -> TriangleBvh {
        let n = self.triangles.len();
        if n == 0 {
            return TriangleBvh {
                nodes: Vec::new(),
                triangle_order: Vec::new(),
                positions: self.positions.to_vec(),
                triangles: self.triangles.to_vec(),
            };
        }
        self.subdivide(0, 0, n);
        TriangleBvh {
            nodes: self.nodes,
            triangle_order: self.triangle_order,
            positions: self.positions.to_vec(),
            triangles: self.triangles.to_vec(),
        }
    }

    fn subdivide(&mut self, node_index: usize, first: usize, count: usize) {
        let bounds = self.range_bounds(first, count);

        if count <= 4 {
            self.nodes[node_index] = BvhNode {
                bounds,
                first: first as u32,
                count: count as u32,
                left_child: 0,
            };
            return;
        }

        let axis = self.longest_centroid_axis(first, count);

        let mid = count / 2;
        self.triangle_order[first..first + count].select_nth_unstable_by(mid, |a, b| {
            let ca = centroid(self.positions, self.triangles[*a as usize]);
            let cb = centroid(self.positions, self.triangles[*b as usize]);
            let va = axis_component(axis, ca);
            let vb = axis_component(axis, cb);
            va.total_cmp(&vb)
        });

        let left_child = self.nodes.len();
        self.nodes.push(BvhNode::placeholder());
        self.nodes.push(BvhNode::placeholder());

        self.nodes[node_index] = BvhNode {
            bounds,
            first: 0,
            count: 0,
            left_child: left_child as u32,
        };

        self.subdivide(left_child, first, mid);
        self.subdivide(left_child + 1, first + mid, count - mid);
    }

    fn range_bounds(&self, first: usize, count: usize) -> Aabb {
        let slice = &self.triangle_order[first..first + count];
        let points: Vec<Vector3<f32>> = slice
            .iter()
            .flat_map(|&idx| {
                let tri = self.triangles[idx as usize];
                [
                    self.positions[tri[0] as usize],
                    self.positions[tri[1] as usize],
                    self.positions[tri[2] as usize],
                ]
            })
            .collect();
        Aabb::from_points(points.into_iter()).unwrap()
    }

    fn longest_centroid_axis(&self, first: usize, count: usize) -> usize {
        let centroids: Vec<Vector3<f32>> = self.triangle_order[first..first + count]
            .iter()
            .map(|&idx| centroid(self.positions, self.triangles[idx as usize]))
            .collect();
        let bounds = Aabb::from_points(centroids.into_iter()).unwrap();
        let dx = bounds.max.x - bounds.min.x;
        let dy = bounds.max.y - bounds.min.y;
        let dz = bounds.max.z - bounds.min.z;
        if dx >= dy && dx >= dz {
            0
        } else if dy >= dz {
            1
        } else {
            2
        }
    }
}

fn centroid(positions: &[Vector3<f32>], tri: [u32; 3]) -> Vector3<f32> {
    (positions[tri[0] as usize] + positions[tri[1] as usize] + positions[tri[2] as usize])
        * (1.0 / 3.0)
}

fn axis_component(axis: usize, v: Vector3<f32>) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

impl TriangleBvh {
    pub fn build(positions: &[Vector3<f32>], triangles: &[[u32; 3]]) -> Self {
        BvhBuilder::new(positions, triangles).build()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NearestTriangle {
    pub triangle: u32,
    pub position: Vector3<f32>,
    pub barycentric: Vector3<f32>,
    pub distance_sq: f32,
}

impl TriangleBvh {
    pub fn nearest(&self, point: Vector3<f32>) -> Option<NearestTriangle> {
        if self.triangles.is_empty() {
            return None;
        }
        let mut best_dist_sq = f32::MAX;
        let mut best_triangle: u32 = 0;
        let mut best_position = Vector3::new(0.0, 0.0, 0.0);
        let mut best_barycentric = Vector3::new(0.0, 0.0, 0.0);
        let mut stack: Vec<u32> = vec![0];
        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx as usize];
            let dist_sq = aabb_distance_sq(&node.bounds, point);
            if dist_sq > best_dist_sq {
                continue;
            }
            if node.count > 0 {
                for i in 0..node.count {
                    let tri_idx = self.triangle_order[(node.first + i) as usize];
                    let tri = self.triangles[tri_idx as usize];
                    let a = self.positions[tri[0] as usize];
                    let b = self.positions[tri[1] as usize];
                    let c = self.positions[tri[2] as usize];
                    let closest = closest_point_on_triangle(point, a, b, c);
                    let d_sq = (closest.position - point).dot(closest.position - point);
                    if d_sq < best_dist_sq {
                        best_dist_sq = d_sq;
                        best_triangle = tri_idx;
                        best_position = closest.position;
                        best_barycentric = closest.barycentric;
                    }
                }
            } else {
                let left = node.left_child;
                let right = left + 1;
                let left_dist = aabb_distance_sq(&self.nodes[left as usize].bounds, point);
                let right_dist = aabb_distance_sq(&self.nodes[right as usize].bounds, point);
                if left_dist <= right_dist {
                    stack.push(right);
                    stack.push(left);
                } else {
                    stack.push(left);
                    stack.push(right);
                }
            }
        }
        Some(NearestTriangle {
            triangle: best_triangle,
            position: best_position,
            barycentric: best_barycentric,
            distance_sq: best_dist_sq,
        })
    }
}

fn aabb_distance_sq(aabb: &Aabb, point: Vector3<f32>) -> f32 {
    let dx = if point.x < aabb.min.x {
        aabb.min.x - point.x
    } else if point.x > aabb.max.x {
        point.x - aabb.max.x
    } else {
        0.0
    };
    let dy = if point.y < aabb.min.y {
        aabb.min.y - point.y
    } else if point.y > aabb.max.y {
        point.y - aabb.max.y
    } else {
        0.0
    };
    let dz = if point.z < aabb.min.z {
        aabb.min.z - point.z
    } else if point.z > aabb.max.z {
        point.z - aabb.max.z
    } else {
        0.0
    };
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triangle_closest_point::closest_point_on_triangle;
    use cgmath::InnerSpace;
    use fastrand::Rng;

    #[test]
    fn test_random_matches_brute_force() {
        let mut rng = Rng::with_seed(7);
        let n_vertices: usize = 300;
        let vertices: Vec<Vector3<f32>> = (0..n_vertices)
            .map(|_| {
                Vector3::new(
                    rng.f32() * 2.0 - 1.0,
                    rng.f32() * 2.0 - 1.0,
                    rng.f32() * 2.0 - 1.0,
                )
            })
            .collect();
        let n_triangles: usize = 200;
        let triangles: Vec<[u32; 3]> = (0..n_triangles)
            .map(|_| {
                [
                    rng.u32(..n_vertices as u32),
                    rng.u32(..n_vertices as u32),
                    rng.u32(..n_vertices as u32),
                ]
            })
            .collect();
        let queries: Vec<Vector3<f32>> = (0..100)
            .map(|_| {
                Vector3::new(
                    rng.f32() * 4.0 - 2.0,
                    rng.f32() * 4.0 - 2.0,
                    rng.f32() * 4.0 - 2.0,
                )
            })
            .collect();
        let bvh = TriangleBvh::build(&vertices, &triangles);
        for p in &queries {
            let bvh_result = bvh.nearest(*p).unwrap();
            let mut best_dist_sq = f32::MAX;
            for tri in &triangles {
                let a = vertices[tri[0] as usize];
                let b = vertices[tri[1] as usize];
                let c = vertices[tri[2] as usize];
                let closest = closest_point_on_triangle(*p, a, b, c);
                let d_sq = (closest.position - *p).dot(closest.position - *p);
                if d_sq < best_dist_sq {
                    best_dist_sq = d_sq;
                }
            }
            assert!(
                (bvh_result.distance_sq - best_dist_sq).abs() < 1e-6,
                "seed=7 query {:?}: bvh={:.8} brute={:.8}",
                p,
                bvh_result.distance_sq,
                best_dist_sq
            );
        }
    }

    #[test]
    fn test_empty_mesh() {
        let bvh = TriangleBvh::build(&[], &[]);
        assert!(bvh.nearest(Vector3::new(0.0, 0.0, 0.0)).is_none());
    }

    #[test]
    fn test_single_triangle() {
        let positions: Vec<Vector3<f32>> = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ];
        let triangles: Vec<[u32; 3]> = vec![[0, 1, 2]];
        let bvh = TriangleBvh::build(&positions, &triangles);
        let point = Vector3::new(0.5, 0.5, 0.5);
        let result = bvh.nearest(point).unwrap();
        let expected = closest_point_on_triangle(point, positions[0], positions[1], positions[2]);
        assert_eq!(result.triangle, 0);
        let pos_diff = result.position - expected.position;
        assert!(pos_diff.dot(pos_diff) < 1e-12);
        let bar_diff = result.barycentric - expected.barycentric;
        assert!(bar_diff.dot(bar_diff) < 1e-12);
    }
}
