use crate::bounds::Aabb;
use crate::coordinate_system::ray_to_triangle_barycentric;
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

#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub triangle: u32,
    pub distance: f32,
    pub barycentric: Vector3<f32>,
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

    pub fn raycast(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        max_distance: f32,
    ) -> Option<RayHit> {
        if self.triangles.is_empty() {
            return None;
        }
        let inverse_direction =
            Vector3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z);
        let mut best_t = max_distance;
        let mut best_triangle: u32 = 0;
        let mut best_barycentric = Vector3::new(0.0, 0.0, 0.0);
        let mut stack: Vec<u32> = vec![0];
        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx as usize];
            if let Some(entry) = ray_aabb_entry(&node.bounds, origin, inverse_direction) {
                if entry >= best_t {
                    continue;
                }
            } else {
                continue;
            }
            if node.count > 0 {
                for i in 0..node.count {
                    let tri_idx = self.triangle_order[(node.first + i) as usize];
                    let tri = self.triangles[tri_idx as usize];
                    let a = self.positions[tri[0] as usize];
                    let b = self.positions[tri[1] as usize];
                    let c = self.positions[tri[2] as usize];
                    if let Some((t, u, v)) = ray_to_triangle_barycentric(origin, direction, a, b, c)
                    {
                        if t < best_t {
                            best_t = t;
                            best_triangle = tri_idx;
                            best_barycentric = Vector3::new(1.0 - u - v, u, v);
                        }
                    }
                }
            } else {
                let left = node.left_child;
                let right = left + 1;
                let left_entry =
                    ray_aabb_entry(&self.nodes[left as usize].bounds, origin, inverse_direction);
                let right_entry = ray_aabb_entry(
                    &self.nodes[right as usize].bounds,
                    origin,
                    inverse_direction,
                );
                match (left_entry, right_entry) {
                    (Some(le), Some(re)) if le <= re => {
                        stack.push(right);
                        stack.push(left);
                    }
                    (Some(_), Some(_)) => {
                        stack.push(left);
                        stack.push(right);
                    }
                    (Some(_), None) => {
                        stack.push(left);
                    }
                    (None, Some(_)) => {
                        stack.push(right);
                    }
                    (None, None) => {}
                }
            }
        }
        if best_t < max_distance {
            Some(RayHit {
                triangle: best_triangle,
                distance: best_t,
                barycentric: best_barycentric,
            })
        } else {
            None
        }
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

fn ray_aabb_entry(
    bounds: &Aabb,
    origin: Vector3<f32>,
    inverse_direction: Vector3<f32>,
) -> Option<f32> {
    let t_min_x = (bounds.min.x - origin.x) * inverse_direction.x;
    let t_max_x = (bounds.max.x - origin.x) * inverse_direction.x;
    let mut t_enter = t_min_x.min(t_max_x);
    let mut t_exit = t_min_x.max(t_max_x);

    let t_min_y = (bounds.min.y - origin.y) * inverse_direction.y;
    let t_max_y = (bounds.max.y - origin.y) * inverse_direction.y;
    t_enter = t_enter.max(t_min_y.min(t_max_y));
    t_exit = t_exit.min(t_min_y.max(t_max_y));

    if t_enter > t_exit {
        return None;
    }

    let t_min_z = (bounds.min.z - origin.z) * inverse_direction.z;
    let t_max_z = (bounds.max.z - origin.z) * inverse_direction.z;
    t_enter = t_enter.max(t_min_z.min(t_max_z));
    t_exit = t_exit.min(t_min_z.max(t_max_z));

    if t_enter > t_exit {
        return None;
    }

    if t_exit < 0.0 {
        return None;
    }

    Some(t_enter)
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

    #[test]
    fn test_raycast_random_matches_brute_force() {
        let mut rng = Rng::with_seed(11);
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
        let queries: Vec<(Vector3<f32>, Vector3<f32>)> = (0..100)
            .map(|_| {
                let origin = Vector3::new(
                    rng.f32() * 4.0 - 2.0,
                    rng.f32() * 4.0 - 2.0,
                    rng.f32() * 4.0 - 2.0,
                );
                let direction = Vector3::new(
                    rng.f32() * 2.0 - 1.0,
                    rng.f32() * 2.0 - 1.0,
                    rng.f32() * 2.0 - 1.0,
                );
                (origin, direction)
            })
            .collect();
        let bvh = TriangleBvh::build(&vertices, &triangles);
        for (origin, direction) in &queries {
            let bvh_result = bvh.raycast(*origin, *direction, f32::MAX);
            let mut best_t: Option<f32> = None;
            for tri in &triangles {
                let a = vertices[tri[0] as usize];
                let b = vertices[tri[1] as usize];
                let c = vertices[tri[2] as usize];
                if let Some((t, _, _)) = ray_to_triangle_barycentric(*origin, *direction, a, b, c) {
                    match best_t {
                        Some(bt) if t < bt => best_t = Some(t),
                        None => best_t = Some(t),
                        _ => {}
                    }
                }
            }
            match (bvh_result, best_t) {
                (Some(hit), Some(bt)) => {
                    assert!(
                        (hit.distance - bt).abs() < 1e-5,
                        "query {:?}/{:?}: bvh={:.8} brute={:.8}",
                        origin,
                        direction,
                        hit.distance,
                        bt
                    );
                }
                (None, None) => {}
                (Some(hit), None) => {
                    panic!(
                        "query {:?}/{:?}: bvh hit triangle {} at {:.8} but brute force found none",
                        origin, direction, hit.triangle, hit.distance
                    );
                }
                (None, Some(bt)) => {
                    panic!(
                        "query {:?}/{:?}: bvh found none but brute force found t={:.8}",
                        origin, direction, bt
                    );
                }
            }
        }
    }

    #[test]
    fn test_raycast_back_facing_only_returns_none() {
        let positions: Vec<Vector3<f32>> = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ];
        let triangles: Vec<[u32; 3]> = vec![[0, 1, 2]];
        let bvh = TriangleBvh::build(&positions, &triangles);
        let origin = Vector3::new(0.5, 0.5, -1.0);
        let direction = Vector3::new(0.0, 0.0, -1.0);
        assert!(bvh.raycast(origin, direction, f32::MAX).is_none());
    }

    #[test]
    fn test_raycast_max_distance_clips() {
        let positions: Vec<Vector3<f32>> = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ];
        let triangles: Vec<[u32; 3]> = vec![[0, 1, 2]];
        let bvh = TriangleBvh::build(&positions, &triangles);
        let origin = Vector3::new(0.5, 0.5, -10.0);
        let direction = Vector3::new(0.0, 0.0, 1.0);
        assert!(bvh.raycast(origin, direction, 5.0).is_none());
        let hit = bvh.raycast(origin, direction, f32::MAX).unwrap();
        assert!((hit.distance - 10.0).abs() < 1e-5);
    }
}
