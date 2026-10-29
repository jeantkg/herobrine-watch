//! A single timed face overlay, sampled from the supplied skin without image bindings.
use crate::{config::Config, logic::Random};
use mod_api::bindings::cinnabar::extension::render;
pub const NAME: &str = "herobrine-face";

pub fn shader() -> String {
    let values = crate::model::face_pixels()
        .iter()
        .map(|v| format!("{v}u"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"const FACE: array<u32,64> = array<u32,64>({values});
fn effect(uv: vec2<f32>) -> vec3<f32> {{
    let size = min(frame.resolution.x, frame.resolution.y) * (1.0 + param(0u) * 0.08);
    let p = (uv - vec2<f32>(0.5)) * frame.resolution.xy / size + vec2<f32>(0.5);
    if any(p < vec2<f32>(0.0)) || any(p >= vec2<f32>(1.0)) {{ return vec3<f32>(0.0); }}
    let xy = min(vec2<u32>(p * 8.0), vec2<u32>(7u));
    let packed = FACE[xy.y * 8u + xy.x];
    let rgb = vec3<f32>(f32(packed & 255u), f32((packed >> 8u) & 255u), f32((packed >> 16u) & 255u)) / 255.0;
    return select(rgb / 12.92, pow((rgb + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4)), rgb > vec3<f32>(0.04045));
}}"#
    )
}
#[derive(Default)]
pub struct Schedule {
    wait: f32,
    remaining: f32,
}
impl Schedule {
    pub fn reset(&mut self, c: &Config, r: &mut Random) {
        self.wait = r.range(c.jumpscare_min_seconds, c.jumpscare_max_seconds);
        self.remaining = 0.;
    }
    pub fn advance(&mut self, dt: f32, c: &Config, r: &mut Random) -> Option<f32> {
        if !c.jumpscares_enabled {
            return None;
        }
        if self.remaining > 0. {
            self.remaining = (self.remaining - dt).max(0.);
            if self.remaining == 0. {
                self.reset(c, r);
                return None;
            }
        } else {
            self.wait -= dt;
            if self.wait > 0. {
                return None;
            }
            self.remaining = c.jumpscare_duration;
        }
        Some(1. - self.remaining / c.jumpscare_duration)
    }
}
#[derive(Default)]
pub struct Effect {
    schedule: Schedule,
    registered: bool,
    active: bool,
}
impl Effect {
    pub fn reset(&mut self, c: &Config, r: &mut Random) {
        self.hide();
        self.registered = false;
        self.schedule.reset(c, r);
    }
    pub fn cancel(&mut self, c: &Config, r: &mut Random) {
        self.hide();
        self.schedule.reset(c, r);
    }
    pub fn hide(&mut self) {
        if self.active {
            let _ = render::update_pass(NAME, false, &[]);
        }
        self.active = false;
    }
    pub fn frame(&mut self, dt: f32, c: &Config, r: &mut Random) -> Result<(), String> {
        if !c.jumpscares_enabled {
            self.hide();
            return Ok(());
        }
        if !self.registered {
            render::register_pass(&render::PassSpec {
                name: NAME.into(),
                order: 90,
                source: shader(),
                depth: false,
            })?;
            render::update_pass(NAME, false, &[])?;
            self.registered = true;
        }
        if let Some(progress) = self.schedule.advance(dt, c, r) {
            render::update_pass(NAME, true, &[progress])?;
            self.active = true;
        } else {
            self.hide();
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn face_jumpscare_is_single_timed_event_and_disabled_mode_stays_off() {
        let mut c = Config::default();
        c.jumpscare_min_seconds = 0.2;
        c.jumpscare_max_seconds = 0.2;
        c.jumpscare_duration = 0.2;
        let mut r = Random::new(1);
        let mut state = Schedule::default();
        state.reset(&c, &mut r);
        assert!(state.advance(0.1, &c, &mut r).is_none());
        assert_eq!(state.advance(0.1, &c, &mut r), Some(0.));
        assert!(state.advance(0.1, &c, &mut r).is_some());
        assert!(state.advance(0.1, &c, &mut r).is_none());
        c.jumpscares_enabled = false;
        for _ in 0..10 {
            assert!(state.advance(1., &c, &mut r).is_none());
        }
    }
    #[test]
    fn face_uses_supplied_skin_and_contains_white_eyes() {
        let face = crate::model::face_pixels();
        assert_eq!(face.len(), 64);
        assert!(face.contains(&0x00ffffff));
    }
}
