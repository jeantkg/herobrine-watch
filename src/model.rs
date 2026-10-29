//! Classic 64x64 player skin layout, rendered as flat-coloured texel rectangles.
//! The current public drawing API does not expose native player/texture creation.
use mod_api::drawing::api::{GeometrySpec, Mesh, Vertex};
use std::collections::HashMap;

const SIDE: usize = 64;
const PIXEL: f32 = 1.8 / 32.;
pub const QUADS_PER_UPLOAD: usize = 48;
const SKIN: &[u8; SIDE * SIDE * 4] = include_bytes!("../assets/herobrine.rgba");

pub fn face_pixels() -> Vec<u32> {
    (8..16)
        .flat_map(|y| {
            (8..16).map(move |x| {
                let p = rgba(x, y);
                u32::from(p[0]) | (u32::from(p[1]) << 8) | (u32::from(p[2]) << 16)
            })
        })
        .collect()
}

/// A closed invisible mask provides the private depth buffer needed by opaque 3D faces.
pub fn depth_mask() -> GeometrySpec {
    let positions = [
        [-0.53, -0.03, -0.53],
        [0.53, -0.03, -0.53],
        [0.53, 1.86, -0.53],
        [-0.53, 1.86, -0.53],
        [-0.53, -0.03, 0.53],
        [0.53, -0.03, 0.53],
        [0.53, 1.86, 0.53],
        [-0.53, 1.86, 0.53],
    ];
    GeometrySpec::Triangles(Mesh {
        vertices: positions
            .into_iter()
            .map(|p| Vertex {
                position: p.into(),
                color: (1., 1., 1., 1.),
                uv: (0., 0.),
                data: (0., 0., 0., 0.),
            })
            .collect(),
        indices: vec![
            0, 3, 2, 0, 2, 1, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 3, 7, 6, 3, 6, 2, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    })
}

#[derive(Clone, Copy)]
struct Part {
    min: [f32; 3],
    size: [usize; 3],
    uv: [usize; 2],
    overlay: [usize; 2],
}
// Standard classic (four-pixel arm) skin boxes. Coordinates are model pixels.
const PARTS: [Part; 6] = [
    Part {
        min: [-4., 24., -4.],
        size: [8, 8, 8],
        uv: [0, 0],
        overlay: [32, 0],
    },
    Part {
        min: [-4., 12., -2.],
        size: [8, 12, 4],
        uv: [16, 16],
        overlay: [16, 32],
    },
    Part {
        min: [-8., 12., -2.],
        size: [4, 12, 4],
        uv: [40, 16],
        overlay: [40, 32],
    },
    Part {
        min: [4., 12., -2.],
        size: [4, 12, 4],
        uv: [32, 48],
        overlay: [48, 48],
    },
    Part {
        min: [-4., 0., -2.],
        size: [4, 12, 4],
        uv: [0, 16],
        overlay: [0, 32],
    },
    Part {
        min: [0., 0., -2.],
        size: [4, 12, 4],
        uv: [16, 48],
        overlay: [0, 48],
    },
];
#[derive(Clone, Copy)]
struct Face {
    origin: [f32; 3],
    u: [f32; 3],
    v: [f32; 3],
    normal: [f32; 3],
    uv: [usize; 2],
    size: [usize; 2],
}
fn faces(part: Part, outer: bool, head: bool) -> [Face; 6] {
    let inflate = if !outer {
        0.
    } else if head {
        0.5
    } else {
        0.25
    };
    let low = part.min.map(|p| p - inflate);
    let high: [f32; 3] = std::array::from_fn(|i| part.min[i] + part.size[i] as f32 + inflate);
    let [x, y, z] = low;
    let [a, b, c] = high;
    let [w, h, d] = part.size;
    let [u, v] = if outer { part.overlay } else { part.uv };
    [
        Face {
            origin: [x, b, z],
            u: [0., 0., c - z],
            v: [0., y - b, 0.],
            normal: [-1., 0., 0.],
            uv: [u, v + d],
            size: [d, h],
        },
        Face {
            origin: [x, b, c],
            u: [a - x, 0., 0.],
            v: [0., y - b, 0.],
            normal: [0., 0., 1.],
            uv: [u + d, v + d],
            size: [w, h],
        },
        Face {
            origin: [a, b, c],
            u: [0., 0., z - c],
            v: [0., y - b, 0.],
            normal: [1., 0., 0.],
            uv: [u + d + w, v + d],
            size: [d, h],
        },
        Face {
            origin: [a, b, z],
            u: [x - a, 0., 0.],
            v: [0., y - b, 0.],
            normal: [0., 0., -1.],
            uv: [u + 2 * d + w, v + d],
            size: [w, h],
        },
        Face {
            origin: [x, b, z],
            u: [a - x, 0., 0.],
            v: [0., 0., c - z],
            normal: [0., 1., 0.],
            uv: [u + d, v],
            size: [w, d],
        },
        Face {
            origin: [x, y, z],
            u: [a - x, 0., 0.],
            v: [0., 0., c - z],
            normal: [0., -1., 0.],
            uv: [u + d + w, v],
            size: [w, d],
        },
    ]
}
fn rgba(x: usize, y: usize) -> [u8; 4] {
    SKIN[(y * SIDE + x) * 4..(y * SIDE + x + 1) * 4]
        .try_into()
        .unwrap()
}
fn linear(color: [u8; 4]) -> [f32; 4] {
    std::array::from_fn(|i| {
        let value = f32::from(color[i]) / 255.;
        if i == 3 {
            value
        } else if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    })
}
fn rectangles(face: Face) -> Vec<([usize; 4], [u8; 4])> {
    let [w, h] = face.size;
    let [u, v] = face.uv;
    let mut used = vec![false; w * h];
    let mut result = vec![];
    for y in 0..h {
        for x in 0..w {
            let color = rgba(u + x, v + y);
            if used[y * w + x] || color[3] == 0 {
                continue;
            }
            let mut width = 1;
            while x + width < w && !used[y * w + x + width] && rgba(u + x + width, v + y) == color {
                width += 1;
            }
            let mut height = 1;
            while y + height < h
                && (x..x + width)
                    .all(|px| !used[(y + height) * w + px] && rgba(u + px, v + y + height) == color)
            {
                height += 1;
            }
            for row in y..y + height {
                used[row * w + x..row * w + x + width].fill(true);
            }
            result.push(([x, y, width, height], color));
        }
    }
    result
}
fn quad(mesh: &mut Mesh, face: Face, rect: [usize; 4], color: [f32; 4]) {
    let [x, y, w, h] = rect;
    let offset = mesh.vertices.len() as u32;
    for (px, py) in [(x, y), (x + w, y), (x + w, y + h), (x, y + h)] {
        let p: [f32; 3] = std::array::from_fn(|i| {
            (face.origin[i]
                + face.u[i] * px as f32 / face.size[0] as f32
                + face.v[i] * py as f32 / face.size[1] as f32)
                * PIXEL
        });
        mesh.vertices.push(Vertex {
            position: (p[0], p[1], p[2]),
            color: color.into(),
            uv: (
                (face.uv[0] + px) as f32 / SIDE as f32,
                (face.uv[1] + py) as f32 / SIDE as f32,
            ),
            data: (0., 0., 0., 0.),
        });
    }
    let cross = [
        face.u[1] * face.v[2] - face.u[2] * face.v[1],
        face.u[2] * face.v[0] - face.u[0] * face.v[2],
        face.u[0] * face.v[1] - face.u[1] * face.v[0],
    ];
    let outward = cross
        .iter()
        .zip(face.normal)
        .map(|(a, b)| a * b)
        .sum::<f32>()
        > 0.;
    let indices = if outward {
        [0, 1, 2, 0, 2, 3]
    } else {
        [0, 2, 1, 0, 3, 2]
    };
    mesh.indices.extend(indices.map(|i| offset + i));
}
/// Builds immutable batches during the larger init allowance, then uploads one per frame.
pub fn build() -> Vec<GeometrySpec> {
    let mut meshes = vec![];
    let mut mesh = Mesh {
        vertices: vec![],
        indices: vec![],
    };
    let mut colors = HashMap::new();
    for (index, part) in PARTS.into_iter().enumerate() {
        for outer in [false, true] {
            for face in faces(part, outer, index == 0) {
                for (rect, color) in rectangles(face) {
                    if mesh.vertices.len() == QUADS_PER_UPLOAD * 4 {
                        meshes.push(GeometrySpec::Triangles(mesh));
                        mesh = Mesh {
                            vertices: vec![],
                            indices: vec![],
                        };
                    }
                    let converted = *colors.entry(color).or_insert_with(|| linear(color));
                    quad(&mut mesh, face, rect, converted);
                }
            }
        }
    }
    if !mesh.vertices.is_empty() {
        meshes.push(GeometrySpec::Triangles(mesh));
    }
    meshes
}

/// Ray intersection with the six standard player boxes in the skin mesh's pose.
pub fn gaze_point(
    eye: [f32; 3],
    yaw: f32,
    pitch: f32,
    feet: [f32; 3],
    facing_yaw: f32,
) -> Option<[f32; 3]> {
    let direction = crate::logic::forward(yaw, pitch);
    let (s, c) = facing_yaw.sin_cos();
    let transform = |p: [f32; 3]| [p[0] * c - p[2] * s, p[1], p[0] * s + p[2] * c];
    let origin = transform(std::array::from_fn(|i| eye[i] - feet[i]));
    let ray = transform(direction);
    let mut closest = f32::INFINITY;
    for part in PARTS {
        let min = part.min.map(|v| v * PIXEL);
        let max: [f32; 3] = std::array::from_fn(|i| (part.min[i] + part.size[i] as f32) * PIXEL);
        let (mut near, mut far) = (0_f32, 128_f32);
        for i in 0..3 {
            if ray[i].abs() < 1e-6 {
                if origin[i] < min[i] || origin[i] > max[i] {
                    far = -1.;
                    break;
                }
            } else {
                let a = (min[i] - origin[i]) / ray[i];
                let b = (max[i] - origin[i]) / ray[i];
                near = near.max(a.min(b));
                far = far.min(a.max(b));
            }
        }
        if near <= far && near < closest {
            closest = near;
        }
    }
    closest
        .is_finite()
        .then(|| std::array::from_fn(|i| eye[i] + direction[i] * closest))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn texel_rectangles_cover_each_nontransparent_skin_pixel_once() {
        for (i, part) in PARTS.into_iter().enumerate() {
            for outer in [false, true] {
                for face in faces(part, outer, i == 0) {
                    let [w, h] = face.size;
                    let mut coverage = vec![0; w * h];
                    for ([x, y, rw, rh], color) in rectangles(face) {
                        for py in y..y + rh {
                            for px in x..x + rw {
                                assert_eq!(rgba(face.uv[0] + px, face.uv[1] + py), color);
                                coverage[py * w + px] += 1;
                            }
                        }
                    }
                    for y in 0..h {
                        for x in 0..w {
                            assert_eq!(
                                coverage[y * w + x],
                                i32::from(rgba(face.uv[0] + x, face.uv[1] + y)[3] != 0)
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn classic_model_bounds_upload_budget_and_white_eyes_are_preserved() {
        let model = build();
        assert!(!model.is_empty());
        assert!(model.len() + 4 < mod_api::MAX_RENDER_CALLS);
        let mut white_front = 0;
        for part in model {
            let GeometrySpec::Triangles(mesh) = part else {
                panic!()
            };
            assert!(mesh.vertices.len() <= QUADS_PER_UPLOAD * 4);
            assert!(
                mesh.indices
                    .iter()
                    .all(|i| (*i as usize) < mesh.vertices.len())
            );
            for vertex in mesh.vertices {
                assert!(vertex.position.1 >= 0. && vertex.position.1 <= 1.801);
                if vertex.color == (1., 1., 1., 1.)
                    && vertex.position.2 > 0.22
                    && vertex.position.1 > 1.35
                {
                    white_front += 1;
                }
            }
        }
        assert!(white_front >= 8);
    }
    #[test]
    fn front_top_and_bottom_skin_faces_match_classic_layout() {
        let faces = faces(PARTS[0], false, true);
        assert_eq!(faces[1].uv, [8, 8]);
        assert_eq!(faces[1].normal, [0., 0., 1.]);
        assert_eq!(faces[4].uv, [8, 0]);
        assert_eq!(faces[5].uv, [16, 0]);
        assert!(faces[5].v[2] > 0.);
    }
    #[test]
    fn direct_gaze_hits_head_body_and_legs_but_not_nearby_air() {
        for y in [0.3, 0.95, 1.6] {
            assert!(gaze_point([0., y, 8.], 0., 0., [0.; 3], 0.).is_some());
        }
        assert!(gaze_point([0.8, 1.6, 8.], 0., 0., [0.; 3], 0.).is_none());
        assert!(gaze_point([0., 1.6, 8.], std::f32::consts::PI, 0., [0.; 3], 0.).is_none());
    }
    #[test]
    fn third_person_gaze_uses_rendered_body_rotation_instead_of_camera_facing() {
        let player = [10., 0., 0.];
        let feet = [0.; 3];
        let camera = [0.4, 0.95, 10.];
        let facing = crate::logic::face(player, feet);
        assert!(gaze_point(camera, 0., 0., feet, facing).is_none());
        assert!(gaze_point(camera, 0., 0., feet, 0.).is_some());
    }
    #[test]
    fn depth_mask_is_closed_and_encloses_every_body_rotation() {
        let GeometrySpec::Triangles(mask) = depth_mask() else {
            panic!()
        };
        let mut edges = HashMap::new();
        for triangle in mask.indices.chunks_exact(3) {
            for (a, b) in [
                (triangle[0], triangle[1]),
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
            ] {
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        assert!(edges.values().all(|count| *count == 2));
        let min: [f32; 3] = std::array::from_fn(|i| {
            mask.vertices
                .iter()
                .map(|v| <[f32; 3]>::from(v.position)[i])
                .fold(f32::INFINITY, f32::min)
        });
        let max: [f32; 3] = std::array::from_fn(|i| {
            mask.vertices
                .iter()
                .map(|v| <[f32; 3]>::from(v.position)[i])
                .fold(f32::NEG_INFINITY, f32::max)
        });
        for geometry in build() {
            let GeometrySpec::Triangles(mesh) = geometry else {
                panic!()
            };
            for vertex in mesh.vertices {
                for angle in 0..360 {
                    let (s, c) = (angle as f32).to_radians().sin_cos();
                    let (x, y, z) = vertex.position;
                    let p = [x * c + z * s, y, z * c - x * s];
                    assert!((0..3).all(|i| p[i] > min[i] && p[i] < max[i]));
                }
            }
        }
    }
}
