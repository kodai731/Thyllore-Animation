use crate::bounds::Aabb;
use crate::coordinate_system::ray_to_triangle_barycentric;
use cgmath::Vector3;

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

impl TriangleBvh {
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
        let mut nearest: Option<RayHit> = None;
        let mut stack: Vec<u32> = vec![0];

        while let Some(node_index) = stack.pop() {
            let node = &self.nodes[node_index as usize];
            let limit = nearest.map_or(max_distance, |hit| hit.distance);
            let Some(entry) = ray_aabb_entry(&node.bounds, origin, inverse_direction) else {
                continue;
            };
            if entry >= limit {
                continue;
            }
            if node.count > 0 {
                if let Some(hit) = self.nearest_leaf_hit(node, origin, direction, limit) {
                    nearest = Some(hit);
                }
            } else {
                self.push_children_nearest_first(node, origin, inverse_direction, &mut stack);
            }
        }
        nearest
    }

    /// The closest triangle of a leaf hit before `limit`, if any.
    fn nearest_leaf_hit(
        &self,
        leaf: &BvhNode,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        limit: f32,
    ) -> Option<RayHit> {
        let mut nearest: Option<RayHit> = None;
        for slot in leaf.first..leaf.first + leaf.count {
            let triangle = self.triangle_order[slot as usize];
            let [a, b, c] = self.triangles[triangle as usize].map(|v| self.positions[v as usize]);
            let Some((distance, u, v)) = ray_to_triangle_barycentric(origin, direction, a, b, c)
            else {
                continue;
            };
            if distance < nearest.map_or(limit, |hit| hit.distance) {
                nearest = Some(RayHit {
                    triangle,
                    distance,
                    barycentric: Vector3::new(1.0 - u - v, u, v),
                });
            }
        }
        nearest
    }

    /// Pushes the children the ray enters, the nearer one on top so it is visited first.
    fn push_children_nearest_first(
        &self,
        node: &BvhNode,
        origin: Vector3<f32>,
        inverse_direction: Vector3<f32>,
        stack: &mut Vec<u32>,
    ) {
        let left = node.left_child;
        let right = left + 1;
        let entry_of = |child: u32| {
            ray_aabb_entry(
                &self.nodes[child as usize].bounds,
                origin,
                inverse_direction,
            )
        };
        match (entry_of(left), entry_of(right)) {
            (Some(left_entry), Some(right_entry)) if left_entry <= right_entry => {
                stack.push(right);
                stack.push(left);
            }
            (Some(_), Some(_)) => {
                stack.push(left);
                stack.push(right);
            }
            (Some(_), None) => stack.push(left),
            (None, Some(_)) => stack.push(right),
            (None, None) => {}
        }
    }
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
    use fastrand::Rng;

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
