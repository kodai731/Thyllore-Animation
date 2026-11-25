fn find_root(parent: &mut [usize], vertex: usize) -> usize {
    let mut root = vertex;
    while parent[root] != root {
        root = parent[root];
    }
    let mut current = vertex;
    while parent[current] != root {
        let next = parent[current];
        parent[current] = root;
        current = next;
    }
    root
}

/// The connected component of every vertex, labelled by the smallest vertex index it contains.
pub fn triangle_component_labels(vertex_count: usize, triangles: &[[u32; 3]]) -> Vec<usize> {
    let mut parent: Vec<usize> = (0..vertex_count).collect();
    for triangle in triangles {
        for edge in [(triangle[0], triangle[1]), (triangle[1], triangle[2])] {
            let a = find_root(&mut parent, edge.0 as usize);
            let b = find_root(&mut parent, edge.1 as usize);
            parent[a.max(b)] = a.min(b);
        }
    }
    (0..vertex_count)
        .map(|vertex| find_root(&mut parent, vertex))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_triangles_sharing_no_vertex_are_two_components() {
        let labels = triangle_component_labels(7, &[[0, 1, 2], [4, 5, 6]]);
        assert_eq!(labels, vec![0, 0, 0, 3, 4, 4, 4]);
    }

    #[test]
    fn triangles_sharing_a_vertex_merge() {
        let labels = triangle_component_labels(5, &[[0, 1, 2], [2, 3, 4]]);
        assert!(labels.iter().all(|&label| label == 0));
    }
}
