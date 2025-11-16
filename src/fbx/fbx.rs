/*
reference from bevy_mod_fbx, FizzWizZleDazzle
https://github.com/FizzWizZleDazzle/bevy_mod_fbx/blob/main/src/loader.rs#L217
 */
use crate::log;
use crate::math::math::*;
use anyhow::{anyhow, Context, Result};
use cgmath::{Matrix4, Quaternion, Deg, Rad, EuclideanSpace, Point3};
use fbxcel::tree::v7400::NodeHandle;
use fbxcel_dom::any::AnyDocument;
use fbxcel_dom::v7400::data::{
    mesh::{
        layer::TypedLayerElementHandle, ControlPointIndex, PolygonVertexIndex, PolygonVertices,
    },
    texture::WrapMode,
};
use fbxcel_dom::v7400::object::property::loaders::StrictF64Loader;
use fbxcel_dom::v7400::{
    object::{
        self,
        model::{ModelHandle, TypedModelHandle},
        ObjectHandle, ObjectId, TypedObjectHandle,
    },
    Document,
};

/// Get local transform matrix from FBX model node
fn get_local_transform(model: &ModelHandle) -> Matrix4<f32> {
    let mut translation = [0.0, 0.0, 0.0];
    let mut rotation = [0.0, 0.0, 0.0];
    let mut scaling = [1.0, 1.0, 1.0];

    let loader = StrictF64Loader;

    // TODO: Figure out the correct fbxcel-dom API to extract property values
    // For now, we'll use a workaround to test if transform application fixes the issue

    // Check if this mesh has a specific name pattern and apply known transforms
    // This is a temporary solution to validate the approach
    let mesh_name = model.name().unwrap_or("");

    // You can manually set transforms for specific meshes here for testing
    // For example, if Blender shows a parent transform of (0, -1.023, 0.0089):
    // Uncomment and adjust these values based on what you see in Blender:
    if mesh_name.contains("BezierCircle") {
        translation = [0.0, -0.81, 0.0];
        rotation = [-90.0, 0.0, 0.0];
        scaling = [1.0, 1.0, 1.0];
    }

    if mesh_name.contains("NurbsPath.001") {
        translation = [0.0, -1.023, 0.008889];
        rotation = [-90.0, 1.1644, 90.0];
        scaling = [1.0, 1.0, 1.0];
    }

    if mesh_name.contains("NurbsPath.002") {
        translation = [0.0, -1.0229, 0.009046];
        rotation = [90.0, 1.5574, 90.0];
        scaling = [1.0, 1.0, 1.0];
    }

    if mesh_name.eq("NurbsPath") {
        translation = [0.0, 0.0, 0.0];
        rotation = [-90.0, 90.0, 0.0];
        scaling = [1.0, 1.0, 1.0];
    }

    log!(
        "Mesh: {}, Using transform - Translation: {:?}, Rotation: {:?}, Scaling: {:?}",
        mesh_name,
        translation,
        rotation,
        scaling
    );

    // Build transform matrix: T * R * S
    let translation_matrix = Matrix4::from_translation(vec3(
        translation[0] as f32,
        translation[1] as f32,
        translation[2] as f32,
    ));

    // Rotation in degrees, convert to radians
    // FBX uses Euler XYZ order by default, so apply rotations in X * Y * Z order
    let rotation_x = Matrix4::from_angle_x(Deg(rotation[0] as f32));
    let rotation_y = Matrix4::from_angle_y(Deg(rotation[1] as f32));
    let rotation_z = Matrix4::from_angle_z(Deg(rotation[2] as f32));
    let rotation_matrix = rotation_x * rotation_y * rotation_z;  // Changed order to X * Y * Z

    let scale_matrix = Matrix4::from_nonuniform_scale(
        scaling[0] as f32,
        scaling[1] as f32,
        scaling[2] as f32,
    );

    // Coordinate system conversion: FBX (Y-up) to Vulkan
    // You may need to adjust this based on your Vulkan coordinate setup
    // Uncomment the line below if you need Y-up to Z-up conversion:
    // let coord_convert = Matrix4::from_angle_x(Deg(-90.0));
    // coord_convert * translation_matrix * rotation_matrix * scale_matrix

    translation_matrix * rotation_matrix * scale_matrix
}

/// Get world transform matrix by traversing parent hierarchy
fn get_world_transform(model: &ModelHandle, _doc: &Document) -> Matrix4<f32> {
    // For now, just return local transform
    // TODO: Traverse parent hierarchy if needed
    get_local_transform(model)
}

pub unsafe fn load_fbx(path: &str) -> anyhow::Result<(FbxModel)> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut fbx_model = FbxModel::default();
    // TODO: multi FbxData per material
    fbx_model.fbx_data.push(FbxData::new());
    match AnyDocument::from_reader(reader).expect("failed to load FBX document") {
        AnyDocument::V7400(fbx_ver, doc) => {
            for object in doc.objects() {
                if let TypedObjectHandle::Model(TypedModelHandle::Mesh(mesh)) = object.get_typed() {
                    log!("Loading mesh {:?}", mesh);
                    let mesh_name = mesh.name().expect("mesh name not found").to_string();
                    log!("mesh node name {}", mesh_name);

                    // Get world transform matrix for this mesh
                    let world_transform = get_world_transform(&mesh, &doc);
                    log!("World transform for {}: {:?}", mesh_name, world_transform);

                    let mesh_handle = mesh.geometry().context("failed to get geometry handle")?;
                    let polygon_vertices = mesh_handle
                        .polygon_vertices()
                        .context("failed to get polygon vertices")?;
                    let triangle_indices = polygon_vertices.triangulate_each(triangulate)?;
                    log!("polygon vertices {:?}", triangle_indices);

                    let mut indices: Vec<u32> = triangle_indices
                        .triangle_vertex_indices()
                        .map(|t| t.to_usize() as u32)
                        .collect();
                    let offset = fbx_model.fbx_data[0].positions.len() as u32;
                    for index in indices.iter_mut() {
                        *index += offset;
                    }
                    log!("indices: count={}, {:?}", indices.len(), indices);
                    fbx_model.fbx_data[0].indices.extend(indices);

                    let get_position =
                        |pos: Option<ControlPointIndex>| -> Result<_, anyhow::Error> {
                            let cpi =
                                pos.ok_or_else(|| anyhow!("failed to get position handle"))?;
                            let point = polygon_vertices.control_point(cpi).ok_or_else(|| {
                                anyhow!("failed to get point handle cpi: {:?}", cpi)
                            })?;

                            // Apply world transform to vertex position
                            let local_pos = Vector3::new(point.x as f32, point.y as f32, point.z as f32);
                            let world_pos = world_transform.transform_point(Point3::from_vec(local_pos));

                            Ok(Vector3::new(world_pos.x, world_pos.y, world_pos.z))
                        };
                    let positions = triangle_indices
                        .iter_control_point_indices()
                        .map(get_position)
                        .collect::<Result<Vec<_>, _>>()
                        .context("failed to get position")?;
                    log!("positions (transformed): {} {:?}", mesh_name, positions);
                    fbx_model.fbx_data[0].positions.extend(positions);
                }
            }
        }
        _ => log!("unsupported FBX version"),
    }
    Ok(fbx_model)
}
pub fn triangulate(
    pvs: &PolygonVertices<'_>,
    poly_pvis: &[PolygonVertexIndex],
    results: &mut Vec<[PolygonVertexIndex; 3]>,
) -> anyhow::Result<()> {
    macro_rules! get_vec {
        ($pvii:expr) => {
            get_vec(pvs, poly_pvis[$pvii])
        };
    }

    match poly_pvis.len() {
        3 => {
            // Got a triangle, no need of triangulation.
            results.push([poly_pvis[0], poly_pvis[1], poly_pvis[2]]);

            Ok(())
        }
        4 => {
            // p0, p1, p2, p3: vertices of the quadrangle (angle{0..3}).
            let p0 = get_vec!(0)?;
            let p1 = get_vec!(1)?;
            let p2 = get_vec!(2)?;
            let p3 = get_vec!(3)?;

            // n1: Normal vector calculated with two edges of the angle1.
            // n3: Normal vector calculated with two edges of the angle3.
            let n1 = (p0 - p1).cross(p1 - p2);
            let n3 = (p2 - p3).cross(p3 - p0);

            // If both angle1 and angle3 are concave, vectors n1 and n3 are
            // oriented in the same direction and `n1.dot(n3)` will be positive.
            // If either angle1 or angle3 is concave, vector n1 and n3 are
            // oriented in the opposite directions and `n1.dot(n3)` will be
            // negative.
            // It does not matter when the vertices of quadrangle is not on the
            // same plane, because whichever diagonal you choose, the cut will
            // be inaccurate.
            if n1.dot(n3) >= 0.0 {
                // Both angle1 and angle3 are concave.
                // This means that either angle0 or angle2 can be convex.
                // Cut from p0 to p2.
                results.extend_from_slice(&[
                    [poly_pvis[0], poly_pvis[1], poly_pvis[2]],
                    [poly_pvis[2], poly_pvis[3], poly_pvis[0]],
                ]);
            } else {
                // Either angle1 or angle3 is convex.
                // Cut from p1 to p3.
                results.extend_from_slice(&[
                    [poly_pvis[0], poly_pvis[1], poly_pvis[3]],
                    [poly_pvis[3], poly_pvis[1], poly_pvis[2]],
                ]);
            }
            Ok(())
        }
        n => {
            let points = (0..n).map(|i| get_vec!(i)).collect::<Result<Vec<_>, _>>()?;
            let points_2d: Vec<_> = {
                // Reduce dimensions for faster computation.
                // This helps treat points which are not on a single plane.
                let (min, max) =
                    bounding_box(&points).expect("Should never happen: there are 5 or more points");

                let width = max - min;

                match smallest_direction(&width) {
                    (x) if x.x > 0.0 => points
                        .into_iter()
                        .map(|v| Vector2::new(v[1], v[2]))
                        .collect(),
                    (x) if x.y > 0.0 => points
                        .into_iter()
                        .map(|v| Vector2::new(v[0], v[2]))
                        .collect(),
                    (x) => points
                        .into_iter()
                        .map(|v| Vector2::new(v[0], v[1]))
                        .collect(),
                }
            };
            // Normal directions.
            let normal_directions = {
                // 0 ... n-1
                let iter_cur = points_2d.iter();

                // n-1, 0, ... n-2
                let iter_prev = points_2d.iter().cycle().skip(n - 1);

                // 1, ... n-1, 0
                let iter_next = points_2d.iter().cycle().skip(1);

                iter_cur
                    .zip(iter_prev)
                    .zip(iter_next)
                    .map(|((cur, prev), next)| {
                        let prev_cur = *prev - *cur;
                        let cur_next = *cur - *next;
                        prev_cur.perp_dot(cur_next) > 0.0
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(normal_directions.len(), n);

            let dirs_true_count = normal_directions.iter().filter(|&&v| v).count();

            if dirs_true_count <= 1 || dirs_true_count >= n - 1 {
                // Zero or one angles are concave.
                let minor_sign = dirs_true_count <= 1;

                // If there are no concave angles, use 0 as center.
                let convex_index = normal_directions
                    .iter()
                    .position(|&sign| sign == minor_sign)
                    .unwrap_or(0);

                let convex_pvi = poly_pvis[convex_index];

                let iter1 = (0..n)
                    .cycle()
                    .skip(convex_index + 1)
                    .take(n - 2)
                    .map(|i| poly_pvis[i]);

                let iter2 = (0..n).cycle().skip(convex_index + 2).map(|i| poly_pvis[i]);

                for (pvi1, pvi2) in iter1.zip(iter2) {
                    results.push([convex_pvi, pvi1, pvi2]);
                }

                Ok(())
            } else {
                log!(
                    "Unsupported polygon: {}-gon with two or more concave angles",
                    n
                );
                Err(anyhow!("Unsupported polygon"))
            }
        }
    }
}

fn get_vec(pvs: &PolygonVertices<'_>, pvi: PolygonVertexIndex) -> anyhow::Result<Vector3<f32>> {
    pvs.control_point(pvi)
        .map(|p| vec3_from_array([p.x as f32, p.y as f32, p.z as f32]))
        .ok_or_else(|| anyhow!("Index out of range: {pvi:?}"))
}

fn bounding_box<'a>(
    points: impl IntoIterator<Item=&'a Vector3<f32>>,
) -> Option<(Vector3<f32>, Vector3<f32>)> {
    points.into_iter().fold(None, |minmax, point| {
        minmax.map_or_else(
            || Some((*point, *point)),
            |(min, max)| {
                Some((
                    Vector3::new(min.x.min(point.x), min.y.min(point.y), min.z.min(point.z)),
                    Vector3::new(max.x.max(point.x), max.y.max(point.y), max.z.max(point.z)),
                ))
            },
        )
    })
}

fn smallest_direction(v: &Vector3<f32>) -> Vector3<f32> {
    match () {
        () if v.x < v.y && v.z < v.x => Vector3::new(0.0, 0.0, 1.0),
        () if v.x < v.y => Vector3::new(1.0, 0.0, 0.0),
        () if v.z < v.y => Vector3::new(0.0, 0.0, 1.0),
        () => Vector3::new(0.0, 1.0, 0.0),
    }
}

#[derive(Clone, Debug, Default)]
pub struct FbxModel {
    pub fbx_data: Vec<FbxData>,
}

#[derive(Clone, Debug)]
pub struct FbxData {
    pub positions: Vec<Vector3<f32>>,
    pub indices: Vec<u32>,
}

impl FbxData {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            indices: Vec::new(),
        }
    }
}
