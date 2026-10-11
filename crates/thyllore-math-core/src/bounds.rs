use cgmath::{InnerSpace, Matrix4, Point3, Transform, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vector3<f32>,
    pub max: Vector3<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingSphere {
    pub center: Vector3<f32>,
    pub radius: f32,
}

impl Aabb {
    pub fn from_points(points: impl IntoIterator<Item = Vector3<f32>>) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next()?;
        let mut aabb = Self {
            min: first,
            max: first,
        };
        for point in points {
            aabb.include(point);
        }
        Some(aabb)
    }

    pub fn include(&mut self, point: Vector3<f32>) {
        self.min = Vector3::new(
            self.min.x.min(point.x),
            self.min.y.min(point.y),
            self.min.z.min(point.z),
        );
        self.max = Vector3::new(
            self.max.x.max(point.x),
            self.max.y.max(point.y),
            self.max.z.max(point.z),
        );
    }

    pub fn corners(&self) -> [Vector3<f32>; 8] {
        let (lo, hi) = (self.min, self.max);
        [
            Vector3::new(lo.x, lo.y, lo.z),
            Vector3::new(hi.x, lo.y, lo.z),
            Vector3::new(lo.x, hi.y, lo.z),
            Vector3::new(hi.x, hi.y, lo.z),
            Vector3::new(lo.x, lo.y, hi.z),
            Vector3::new(hi.x, lo.y, hi.z),
            Vector3::new(lo.x, hi.y, hi.z),
            Vector3::new(hi.x, hi.y, hi.z),
        ]
    }

    pub fn transformed(&self, matrix: &Matrix4<f32>) -> Self {
        let transformed_corners = self.corners().map(|corner| {
            let point = matrix.transform_point(Point3::new(corner.x, corner.y, corner.z));
            Vector3::new(point.x, point.y, point.z)
        });
        Self::from_points(transformed_corners).unwrap_or(*self)
    }

    pub fn bounding_sphere(&self) -> BoundingSphere {
        let center = (self.min + self.max) * 0.5;
        BoundingSphere {
            center,
            radius: (self.max - center).magnitude(),
        }
    }
}

impl BoundingSphere {
    pub fn union(self, other: BoundingSphere) -> BoundingSphere {
        let offset = other.center - self.center;
        let distance = offset.magnitude();

        if distance + other.radius <= self.radius {
            return self;
        }
        if distance + self.radius <= other.radius {
            return other;
        }

        let radius = (distance + self.radius + other.radius) * 0.5;
        let direction = offset / distance;
        BoundingSphere {
            center: self.center + direction * (radius - self.radius),
            radius,
        }
    }

    pub fn union_all(spheres: impl IntoIterator<Item = BoundingSphere>) -> Option<BoundingSphere> {
        spheres.into_iter().reduce(BoundingSphere::union)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{Matrix4, Vector3};

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn aabb_from_points_and_sphere() {
        let aabb = Aabb::from_points([
            Vector3::new(-1.0, 0.0, 2.0),
            Vector3::new(3.0, -2.0, 0.0),
            Vector3::new(0.0, 4.0, 1.0),
        ])
        .unwrap();
        assert_eq!(aabb.min, Vector3::new(-1.0, -2.0, 0.0));
        assert_eq!(aabb.max, Vector3::new(3.0, 4.0, 2.0));

        let sphere = aabb.bounding_sphere();
        assert_eq!(sphere.center, Vector3::new(1.0, 1.0, 1.0));
        assert!(approx(sphere.radius, (4.0f32 + 9.0 + 1.0).sqrt()));
        assert!(Aabb::from_points(std::iter::empty()).is_none());
    }

    #[test]
    fn aabb_transformed_covers_rotated_corners() {
        let aabb = Aabb {
            min: Vector3::new(-1.0, -1.0, -1.0),
            max: Vector3::new(1.0, 1.0, 1.0),
        };
        let rotated = aabb.transformed(&Matrix4::from_angle_y(cgmath::Deg(45.0)));
        assert!(approx(rotated.max.x, 2.0f32.sqrt()));
        assert!(approx(rotated.min.z, -(2.0f32.sqrt())));

        let moved = aabb.transformed(&Matrix4::from_translation(Vector3::new(5.0, 0.0, 0.0)));
        assert_eq!(moved.min, Vector3::new(4.0, -1.0, -1.0));
    }

    #[test]
    fn sphere_union_contains_both_and_is_symmetric() {
        let a = BoundingSphere {
            center: Vector3::new(0.0, 0.0, 0.0),
            radius: 1.0,
        };
        let b = BoundingSphere {
            center: Vector3::new(4.0, 0.0, 0.0),
            radius: 1.0,
        };
        let ab = a.union(b);
        let ba = b.union(a);
        assert_eq!(ab.center, Vector3::new(2.0, 0.0, 0.0));
        assert!(approx(ab.radius, 3.0));
        assert_eq!(ab, ba);

        let inner = BoundingSphere {
            center: Vector3::new(0.5, 0.0, 0.0),
            radius: 0.2,
        };
        assert_eq!(a.union(inner), a);
        assert_eq!(BoundingSphere::union_all([a, b, inner]).unwrap(), ab);
        assert!(BoundingSphere::union_all(std::iter::empty()).is_none());
    }
}
