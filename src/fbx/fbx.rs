/*
reference from bevy_mod_fbx, FizzWizZleDazzle
https://github.com/FizzWizZleDazzle/bevy_mod_fbx/blob/main/src/loader.rs#L217
 */
use crate::log;
use crate::math::math::*;
use anyhow::{anyhow, Context, Result};
use cgmath::{Matrix4, Quaternion};
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

pub unsafe fn load_fbx(path: &str) -> anyhow::Result<()> {
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
                            Ok(Vector3::new(point.x as f32, point.y as f32, point.z as f32))
                        };
                    let positions = triangle_indices
                        .iter_control_point_indices()
                        .map(get_position)
                        .collect::<Result<Vec<_>, _>>()
                        .context("failed to get position")?;
                    log!("positions: {} {:?}", mesh_name, positions);
                    fbx_model.fbx_data[0].positions.extend(positions);
                }
            }
        }
        _ => log!("unsupported FBX version"),
    }
    Ok(())
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
    points: impl IntoIterator<Item = &'a Vector3<f32>>,
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
    fbx_data: Vec<FbxData>,
}

#[derive(Clone, Debug)]
struct FbxData {
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
