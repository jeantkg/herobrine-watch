//! Loaded collision geometry only; missing samples never count as safe air.
use mod_api::bindings::cinnabar::extension::terrain::*;

pub fn column(x: i32, y: i32, z: i32) -> RegionQuery {
    RegionQuery {
        min: Position {
            x: x - 1,
            y: y - 6,
            z: z - 1,
        },
        max: Position {
            x: x + 2,
            y: y + 8,
            z: z + 2,
        },
        geometry: Geometry::COLLISION,
    }
}
pub fn ray_query(start: [f32; 3], end: [f32; 3]) -> Option<RegionQuery> {
    if !start
        .iter()
        .chain(end.iter())
        .all(|v| v.is_finite() && v.abs() < 29_999_980.)
    {
        return None;
    }
    let min: [i32; 3] = std::array::from_fn(|i| start[i].min(end[i]).floor() as i32);
    let max: [i32; 3] = std::array::from_fn(|i| start[i].max(end[i]).floor() as i32 + 1);
    if (0..3).any(|i| max[i] - min[i] > i32::from(mod_api::MAX_TERRAIN_EXTENT)) {
        return None;
    }
    Some(RegionQuery {
        min: Position {
            x: min[0],
            y: min[1],
            z: min[2],
        },
        max: Position {
            x: max[0],
            y: max[1],
            z: max[2],
        },
        geometry: Geometry::COLLISION,
    })
}
/// Renew short ray queries together so distant gaze can resolve asynchronously.
pub fn ray_segments(
    start: [f32; 3],
    end: [f32; 3],
) -> Option<Vec<(RegionQuery, [f32; 3], [f32; 3])>> {
    let length = start
        .iter()
        .zip(end)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        .sqrt();
    if !length.is_finite() {
        return None;
    }
    let count = (length / 12.).ceil().max(1.) as usize;
    if count > mod_api::MAX_TERRAIN_QUERIES {
        return None;
    }
    (0..count)
        .map(|i| {
            let from = std::array::from_fn(|axis| {
                start[axis] + (end[axis] - start[axis]) * i as f32 / count as f32
            });
            let to = std::array::from_fn(|axis| {
                start[axis] + (end[axis] - start[axis]) * (i + 1) as f32 / count as f32
            });
            Some((ray_query(from, to)?, from, to))
        })
        .collect()
}
fn cell(query: RegionQuery, p: [i32; 3]) -> Option<usize> {
    let min = [query.min.x, query.min.y, query.min.z];
    let max = [query.max.x, query.max.y, query.max.z];
    if (0..3).any(|i| p[i] < min[i] || p[i] >= max[i]) {
        return None;
    }
    Some(
        (((p[0] - min[0]) * (max[2] - min[2]) + p[2] - min[2]) * (max[1] - min[1]) + p[1] - min[1])
            as usize,
    )
}
fn boxes(region: &Region, query: RegionQuery, p: [i32; 3], layer: usize) -> Option<&[f64]> {
    let c = cell(query, p)?;
    if region.loaded.get(c / 64)? & (1 << (c % 64)) == 0 {
        return None;
    }
    let count = ((query.max.x - query.min.x)
        * (query.max.y - query.min.y)
        * (query.max.z - query.min.z)) as usize;
    let slot = layer * count + c;
    let palette = *region.states.get(slot)?;
    if palette == u16::MAX {
        return if layer == 0 { None } else { Some(&[]) };
    }
    let state = region.palette.get(palette as usize)?;
    let shape = match state.collision {
        ShapeBinding::Fixed(shape) => shape,
        ShapeBinding::PerCell => {
            region
                .overrides
                .iter()
                .find(|v| v.slot as usize == slot)?
                .collision?
        }
        ShapeBinding::NotRequested => return None,
    };
    let ResolvedShape::Known(shape) = shape else {
        return None;
    };
    let start = *region.shape_offsets.get(shape as usize)? as usize * 6;
    let end = *region.shape_offsets.get(shape as usize + 1)? as usize * 6;
    region.shape_boxes.get(start..end)
}
pub fn standing(region: &Region, query: RegionQuery, preferred_y: i32) -> Option<[f32; 3]> {
    if region.layer_count == 0 {
        return None;
    }
    let x = query.min.x + 1;
    let z = query.min.z + 1;
    let mut result = None;
    let mut distance = i32::MAX;
    for y in query.min.y..query.max.y - 2 {
        let floor = boxes(region, query, [x, y, z], 0)?;
        if !floor.chunks_exact(6).any(|b| b == [0., 0., 0., 1., 1., 1.]) {
            continue;
        }
        let foot = [x as f32 + 0.5, (y + 1) as f32, z as f32 + 0.5];
        let mut clear = true;
        for cy in y + 1..=y + 2 {
            for layer in 0..usize::from(region.layer_count) {
                let Some(shape) = boxes(region, query, [x, cy, z], layer) else {
                    clear = false;
                    continue;
                };
                if !shape.is_empty() {
                    clear = false;
                }
                let c = cell(query, [x, cy, z])?;
                let count = ((query.max.x - query.min.x)
                    * (query.max.y - query.min.y)
                    * (query.max.z - query.min.z)) as usize;
                if let Some(state) = region
                    .states
                    .get(layer * count + c)
                    .and_then(|i| region.palette.get(*i as usize))
                    && state.physics.as_ref().is_some_and(|p| {
                        p.flags.intersects(PhysicsFlags::WATER | PhysicsFlags::LAVA)
                    })
                {
                    clear = false;
                }
            }
        }
        if clear && (y + 1 - preferred_y).abs() < distance {
            distance = (y + 1 - preferred_y).abs();
            result = Some(foot);
        }
    }
    result
}
pub fn clear_ray(region: &Region, query: RegionQuery, start: [f32; 3], end: [f32; 3]) -> bool {
    if region.layer_count == 0 {
        return false;
    }
    let mut position = start.map(|v| v.floor() as i32);
    let destination = end.map(|v| v.floor() as i32);
    let direction: [f32; 3] = std::array::from_fn(|i| end[i] - start[i]);
    let step = direction.map(|v| if v >= 0. { 1 } else { -1 });
    let delta = direction.map(|v| {
        if v.abs() < 1e-6 {
            f32::INFINITY
        } else {
            1. / v.abs()
        }
    });
    let mut next: [f32; 3] = std::array::from_fn(|i| {
        if direction[i].abs() < 1e-6 {
            f32::INFINITY
        } else {
            (position[i] as f32 + if step[i] > 0 { 1. } else { 0. } - start[i]) / direction[i]
        }
    });
    // A valid query spans at most 16 cells on each axis; one axis advances per step.
    for _ in 0..(usize::from(mod_api::MAX_TERRAIN_EXTENT) * 3 + 1) {
        let [x, y, z] = position;
        for layer in 0..usize::from(region.layer_count) {
            let Some(shape) = boxes(region, query, [x, y, z], layer) else {
                return false;
            };
            for b in shape.chunks_exact(6) {
                let origin = [x as f32, y as f32, z as f32];
                let min = std::array::from_fn(|i| origin[i] + b[i] as f32);
                let max = std::array::from_fn(|i| origin[i] + b[i + 3] as f32);
                if crate::logic::intersects(start, end, min, max) {
                    return false;
                }
            }
        }
        if position == destination {
            return true;
        }
        let axis = if next[0] <= next[1] && next[0] <= next[2] {
            0
        } else if next[1] <= next[2] {
            1
        } else {
            2
        };
        position[axis] += step[axis];
        next[axis] += delta[axis];
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(query: RegionQuery) -> Region {
        let state = |name: &str, shape| BlockState {
            runtime_id: shape,
            identifier: Some(name.into()),
            canonical_state: None,
            tags: vec![],
            physics: None,
            collision: ShapeBinding::Fixed(ResolvedShape::Known(shape)),
            selection: ShapeBinding::NotRequested,
        };
        let mut states = vec![];
        for _x in query.min.x..query.max.x {
            for _z in query.min.z..query.max.z {
                for y in query.min.y..query.max.y {
                    states.push(u16::from(y < 64));
                }
            }
        }
        Region {
            stamp: Stamp {
                session: 2,
                dimension: 0,
                registry: 1,
                revision: 1,
            },
            chunks: vec![],
            layer_count: 1,
            loaded: vec![u64::MAX; states.len().div_ceil(64)],
            states,
            palette: vec![state("minecraft:air", 0), state("minecraft:stone", 1)],
            shape_offsets: vec![0, 0, 1],
            shape_boxes: vec![0., 0., 0., 1., 1., 1.],
            overrides: vec![],
        }
    }
    #[test]
    fn placement_requires_known_floor_and_headroom() {
        let query = column(10, 64, 10);
        let mut region = fixture(query);
        assert_eq!(standing(&region, query, 64), Some([10.5, 64., 10.5]));
        region.loaded.fill(0);
        assert!(standing(&region, query, 64).is_none());
        region.loaded.fill(u64::MAX);
        region.states.fill(0);
        assert!(standing(&region, query, 64).is_none());
        region.states.fill(1);
        assert!(standing(&region, query, 64).is_none());
        region.states.fill(u16::MAX);
        assert!(standing(&region, query, 64).is_none());
    }
    #[test]
    fn visibility_requires_loaded_clear_cells_and_rejects_walls() {
        let start = [0.5, 65.5, 0.5];
        let end = [10.5, 65., 10.5];
        let query = ray_query(start, end).unwrap();
        let mut region = fixture(query);
        assert!(clear_ray(&region, query, start, end));
        let index = cell(query, [5, 65, 5]).unwrap();
        region.states[index] = 1;
        assert!(!clear_ray(&region, query, start, end));
        region.states[index] = 0;
        region.loaded.fill(0);
        assert!(!clear_ray(&region, query, start, end));
        assert!(ray_query(start, [100., 65., 0.]).is_none());
    }
}
