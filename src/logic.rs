//! Gameplay-independent randomness, viewing and timing. Time is supplied by the caller.
pub struct Random(u64);
impl Random {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        min + (max - min) * ((x >> 40) as f32 / 16_777_216.)
    }
}
pub fn forward(yaw: f32, pitch: f32) -> [f32; 3] {
    [
        -yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    ]
}
pub fn looking(eye: [f32; 3], yaw: f32, pitch: f32, target: [f32; 3], degrees: f32) -> bool {
    let d = std::array::from_fn::<_, 3, _>(|i| target[i] - eye[i]);
    let length = d.iter().map(|v| v * v).sum::<f32>().sqrt();
    length > 0.001
        && forward(yaw, pitch)
            .iter()
            .zip(d)
            .map(|(a, b)| a * b)
            .sum::<f32>()
            / length
            >= degrees.to_radians().cos()
}
pub fn face(player: [f32; 3], feet: [f32; 3]) -> f32 {
    // The standard player skin faces local +Z.
    (player[0] - feet[0]).atan2(player[2] - feet[2])
}
pub fn dt(seconds: f32) -> f32 {
    if seconds.is_finite() {
        seconds.clamp(0., 0.1)
    } else {
        0.
    }
}

/// Segment/AABB intersection, excluding the last endpoint at the model.
pub fn intersects(start: [f32; 3], end: [f32; 3], min: [f32; 3], max: [f32; 3]) -> bool {
    let (mut near, mut far) = (0.001_f32, 0.999_f32);
    for i in 0..3 {
        let direction = end[i] - start[i];
        if direction.abs() < 1e-6 {
            if start[i] < min[i] || start[i] > max[i] {
                return false;
            }
        } else {
            let a = (min[i] - start[i]) / direction;
            let b = (max[i] - start[i]) / direction;
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return false;
            }
        }
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gaze_uses_actor_yxz_radians_and_pitch() {
        assert!(looking([0.; 3], 0., 0., [0., 0., -10.], 8.));
        assert!(!looking([0.; 3], 0., 0., [0., 0., 10.], 8.));
        assert!(looking(
            [0.; 3],
            std::f32::consts::FRAC_PI_2,
            0.,
            [-10., 0., 0.],
            8.
        ));
        assert!(looking(
            [0.; 3],
            0.,
            std::f32::consts::FRAC_PI_4,
            [0., 10., -10.],
            8.
        ));
        assert_eq!(face([0., 0., 10.], [0.; 3]), 0.);
    }
    #[test]
    fn seeded_randomness_is_bounded_varied_and_repeatable() {
        let mut a = Random::new(5);
        let mut b = Random::new(5);
        let first = a.range(30., 90.);
        assert_eq!(first, b.range(30., 90.));
        let values: Vec<_> = (0..100).map(|_| a.range(30., 90.)).collect();
        assert!(values.iter().all(|v| (30.0..90.0).contains(v)));
        assert!(values.iter().any(|v| *v != first));
        assert_eq!(dt(f32::NAN), 0.);
        assert_eq!(dt(10.), 0.1);
    }
    #[test]
    fn ray_occlusion_distinguishes_walls_and_parallel_misses() {
        assert!(intersects(
            [0., 1., 0.],
            [0., 1., -10.],
            [-1., 0., -5.],
            [1., 2., -4.]
        ));
        assert!(!intersects(
            [0., 1., 0.],
            [0., 1., -10.],
            [2., 0., -5.],
            [3., 2., -4.]
        ));
    }
}
