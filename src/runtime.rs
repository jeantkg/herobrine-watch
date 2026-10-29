use crate::{
    config::Config,
    logic::{self, Random},
};
use mod_api::{
    bindings::cinnabar::extension::{
        gameplay, hud, player_state, settings,
        terrain::{self as api, ReadResult, RegionQuery},
    },
    drawing::{self, api::*},
};

#[derive(PartialEq)]
struct Scope {
    actor: u64,
    connection: u64,
    world: String,
    dimension: i32,
}
enum Phase {
    Wait(f32),
    Search {
        query: RegionQuery,
        elapsed: f32,
        attempts: u8,
    },
    Active {
        feet: [f32; 3],
        gaze: f32,
    },
}
pub struct Runtime {
    config: Config,
    enabled: bool,
    scope: Option<Scope>,
    random: Random,
    phase: Phase,
    model: Vec<GeometrySpec>,
    geometry: Vec<Handle>,
    mask_geometry: Option<Handle>,
    clip: Option<Handle>,
    instances: Vec<Instance>,
    jumpscare: crate::jumpscare::Effect,
    jumpscare_failed: bool,
    shown: bool,
    failed: bool,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            config: Config::default(),
            enabled: false,
            scope: None,
            random: Random::new(1),
            phase: Phase::Wait(1.),
            model: Vec::new(),
            geometry: Vec::new(),
            mask_geometry: None,
            clip: None,
            instances: Vec::new(),
            jumpscare: crate::jumpscare::Effect::default(),
            jumpscare_failed: false,
            shown: false,
            failed: false,
        }
    }
}
pub fn usable(frame: &gameplay::Snapshot, world: &player_state::WorldObservation) -> bool {
    world.alive
        && frame.dimension == world.dimension
        && [
            world.position.0,
            world.position.1,
            world.position.2,
            frame.eye.x,
            frame.eye.y,
            frame.eye.z,
        ]
        .iter()
        .all(|v| v.is_finite() && v.abs() < 29_999_970.)
        && frame.yaw.is_finite()
        && frame.pitch.is_finite()
}
impl Runtime {
    pub fn init(&mut self) {
        match settings::load()
            .map_err(|e| e.to_string())
            .and_then(|s| Config::parse(&s))
        {
            Ok(config) => {
                self.enabled = config.enabled;
                self.config = config;
                if self.enabled {
                    self.model = crate::model::build();
                }
            }
            Err(error) => self.error(&format!("Herobrine settings: {error}")),
        }
    }
    fn error(&self, message: &str) {
        if self.config.show_status {
            let _ = hud::set_label(message);
        }
    }
    fn hide(&mut self) {
        if self.shown {
            let _ = replace_frame_group("herobrine", &[]);
            self.shown = false;
        }
        self.instances.clear();
        if let Some(clip) = self.clip.take() {
            let _ = remove(clip);
        }
    }
    fn clear(&mut self) {
        self.hide();
        self.jumpscare.hide();
        for geometry in self.geometry.drain(..) {
            let _ = remove(geometry);
        }
        if let Some(mask) = self.mask_geometry.take() {
            let _ = remove(mask);
        }
        self.scope = None;
        self.failed = false;
        self.jumpscare_failed = false;
    }
    fn cooldown(&mut self) -> Phase {
        Phase::Wait(self.random.range(
            self.config.min_interval_seconds,
            self.config.max_interval_seconds,
        ))
    }
    fn candidate(&mut self, world: &player_state::WorldObservation, yaw: f32) -> RegionQuery {
        let angle = yaw
            + self.random.range(85_f32.to_radians(), 160_f32.to_radians())
                * if self.random.range(0., 1.) < 0.5 {
                    -1.
                } else {
                    1.
                };
        let radius = self
            .random
            .range(self.config.min_distance, self.config.max_distance);
        crate::terrain::column(
            (world.position.0 - angle.sin() * radius).floor() as i32,
            world.position.1.floor() as i32,
            (world.position.2 - angle.cos() * radius).floor() as i32,
        )
    }
    fn show(
        &mut self,
        world: &player_state::WorldObservation,
        feet: [f32; 3],
    ) -> Result<(), DrawError> {
        let player = [world.position.0, world.position.1, world.position.2];
        let anchor = Anchor::World(WorldAnchor {
            dimension: world.dimension,
            origin: (f64::from(feet[0]), f64::from(feet[1]), f64::from(feet[2])),
        });
        if self.clip.is_none() {
            self.clip = Some(create_clip(ClipSpec {
                geometry: self.mask_geometry.expect("uploaded depth mask"),
                anchor,
                transform: Transform {
                    translation: (0., 0., 0.),
                    rotation: (0., 0., 0., 1.),
                    scale: (1., 1., 1.),
                },
                parent: None,
            })?);
        }
        // Ordinary drawing has no self-depth writes; the clip gives all batches one depth buffer.
        clip_status(self.clip.unwrap())?;
        if self.instances.len() != self.geometry.len() {
            self.instances = self
                .geometry
                .iter()
                .map(|id| drawing::instance(*id, anchor))
                .collect();
        }
        let angle = logic::face(player, feet);
        for instance in &mut self.instances {
            instance.clip = self.clip;
            instance.anchor = anchor;
            instance.transform.rotation = (0., (angle * 0.5).sin(), 0., (angle * 0.5).cos());
        }
        replace_frame_group("herobrine", &self.instances)?;
        self.shown = true;
        Ok(())
    }
    pub fn frame(&mut self) {
        if !self.enabled {
            return;
        }
        let frame = gameplay::read_frame().ok().flatten();
        let world = player_state::read_world().ok().flatten();
        // A menu pauses scare logic but is not a reason to remove a still-valid apparition.
        if frame.is_none()
            && world.as_ref().is_some_and(|w| {
                w.alive
                    && self.scope.as_ref().is_some_and(|s| {
                        s.connection == w.session
                            && s.world == w.world
                            && s.dimension == w.dimension
                    })
            })
        {
            self.jumpscare.cancel(&self.config, &mut self.random);
            if let (Phase::Active { feet, .. }, Some(w)) = (&self.phase, world.as_ref()) {
                let player = [w.position.0, w.position.1, w.position.2];
                if player
                    .iter()
                    .zip(feet)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f32>()
                    > self.config.despawn_distance.powi(2)
                {
                    self.hide();
                    self.phase = self.cooldown();
                }
            }
            if self.shown {
                let _ = replace_frame_group("herobrine", &self.instances);
            }
            return;
        }
        let Some((frame, world)) = frame.zip(world).filter(|(f, w)| usable(f, w)) else {
            self.clear();
            return;
        };
        let scope = Scope {
            actor: frame.session,
            connection: world.session,
            world: world.world.clone(),
            dimension: world.dimension,
        };
        if self.scope.as_ref() != Some(&scope) {
            self.clear();
            self.random = Random::new(
                world.session
                    ^ frame.session.rotate_left(19)
                    ^ world.runtime_id.rotate_left(33)
                    ^ mod_api::player_mod::cinnabar::extension::screen::elapsed_seconds().to_bits(),
            );
            self.phase = Phase::Wait(
                self.random
                    .range(self.config.first_min_seconds, self.config.first_max_seconds),
            );
            self.scope = Some(scope);
            self.jumpscare.reset(&self.config, &mut self.random);
        }
        if self.failed {
            return;
        }
        if self.mask_geometry.is_none() {
            match create_geometry(&crate::model::depth_mask()) {
                Ok(id) => self.mask_geometry = Some(id),
                Err(error) => {
                    self.failed = true;
                    self.error(&format!("Herobrine depth mask: {}", error.detail));
                }
            }
            return;
        }
        // Upload small pieces before starting the scare timer. Never reveal a partial player.
        if self.geometry.len() < self.model.len() {
            match create_geometry(&self.model[self.geometry.len()]) {
                Ok(id) => self.geometry.push(id),
                Err(error) => {
                    self.failed = true;
                    self.error(&format!(
                        "Herobrine model: {:?}: {}",
                        error.code, error.detail
                    ));
                }
            }
            return;
        }
        let camera = gameplay::read_camera().ok().flatten();
        let (eye, yaw, pitch) = camera
            .map(|c| ([c.position.x, c.position.y, c.position.z], c.yaw, c.pitch))
            .unwrap_or((
                [frame.eye.x, frame.eye.y, frame.eye.z],
                frame.yaw,
                frame.pitch,
            ));
        if !eye.iter().all(|v| v.is_finite()) || !yaw.is_finite() || !pitch.is_finite() {
            self.clear();
            return;
        }
        let dt = logic::dt(frame.frame_seconds);
        if !self.jumpscare_failed {
            if let Err(error) = self.jumpscare.frame(dt, &self.config, &mut self.random) {
                self.jumpscare.hide();
                self.jumpscare_failed = true;
                self.error(&format!("Herobrine face effect: {error}"));
            }
        }
        let phase = std::mem::replace(&mut self.phase, Phase::Wait(1.));
        self.phase = match phase {
            Phase::Wait(remaining) if remaining > dt => Phase::Wait(remaining - dt),
            Phase::Wait(_) => Phase::Search {
                query: self.candidate(&world, yaw),
                elapsed: 0.,
                attempts: 0,
            },
            Phase::Search {
                query,
                elapsed,
                attempts,
            } => {
                let mut feet = None;
                let mut retry = false;
                match api::read_region(query, None) {
                    Ok(ReadResult::Changed(region))
                        if region.stamp.session == world.session
                            && region.stamp.dimension == world.dimension =>
                    {
                        feet = crate::terrain::standing(
                            &region,
                            query,
                            world.position.1.floor() as i32,
                        );
                        retry = feet.is_none();
                    }
                    Ok(ReadResult::Pending) | Err(api::ReadError::BudgetExhausted) => {}
                    _ => retry = true,
                }
                if let Some(feet) = feet.filter(|feet| {
                    let distance =
                        (feet[0] - world.position.0).powi(2) + (feet[2] - world.position.2).powi(2);
                    distance >= self.config.min_distance.powi(2)
                        && distance <= self.config.max_distance.powi(2)
                        && !logic::looking(eye, yaw, pitch, [feet[0], feet[1] + 0.95, feet[2]], 35.)
                }) {
                    Phase::Active { feet, gaze: 0. }
                } else if elapsed + dt > 5. || attempts >= 5 {
                    self.cooldown()
                } else if retry || feet.is_some() {
                    Phase::Search {
                        query: self.candidate(&world, yaw),
                        elapsed: elapsed + dt,
                        attempts: attempts + 1,
                    }
                } else {
                    Phase::Search {
                        query,
                        elapsed: elapsed + dt,
                        attempts,
                    }
                }
            }
            Phase::Active { feet, mut gaze } => {
                let player = [world.position.0, world.position.1, world.position.2];
                let visible =
                    crate::model::gaze_point(eye, yaw, pitch, feet, logic::face(player, feet))
                        .and_then(|target| crate::terrain::ray_segments(eye, target))
                        .is_some_and(|segments| {
                            let mut clear = true;
                            for (query, start, end) in segments {
                                clear &= match api::read_region(query, None) {
                                    Ok(ReadResult::Changed(region))
                                        if region.stamp.session == world.session
                                            && region.stamp.dimension == world.dimension =>
                                    {
                                        crate::terrain::clear_ray(&region, query, start, end)
                                    }
                                    _ => false,
                                };
                            }
                            clear
                        });
                gaze = if visible { gaze + dt } else { 0. };
                let player = [world.position.0, world.position.1, world.position.2];
                let distance = player
                    .iter()
                    .zip(feet)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f32>();
                if gaze >= self.config.gaze_seconds && gaze > 0.
                    || distance > self.config.despawn_distance.powi(2)
                {
                    self.hide();
                    self.cooldown()
                } else {
                    Phase::Active { feet, gaze }
                }
            }
        };
        if let Phase::Active { feet, .. } = self.phase {
            if let Err(error) = self.show(&world, feet) {
                self.hide();
                for geometry in self.geometry.drain(..) {
                    let _ = remove(geometry);
                }
                self.failed = true;
                self.error(&format!(
                    "Herobrine drawing: {:?}: {}",
                    error.code, error.detail
                ));
            }
        } else {
            self.hide();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_actor_connection_ids_are_accepted_but_dead_and_invalid_frames_are_not() {
        let mut frame = gameplay::Snapshot {
            session: 17,
            dimension: 0,
            eye: gameplay::Vector3 {
                x: 0.,
                y: 65.6,
                z: 0.,
            },
            yaw: 0.,
            pitch: 0.,
            frame_seconds: 0.016,
            players: vec![],
        };
        let mut world = player_state::WorldObservation {
            session: 2,
            dimension: 0,
            world: "fixture".into(),
            position: (0., 64., 0.),
            alive: true,
            runtime_id: 1,
        };
        assert!(usable(&frame, &world));
        world.alive = false;
        assert!(!usable(&frame, &world));
        world.alive = true;
        frame.dimension = 1;
        assert!(!usable(&frame, &world));
        frame.dimension = 0;
        frame.yaw = f32::NAN;
        assert!(!usable(&frame, &world));
    }
}
