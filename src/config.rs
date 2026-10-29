use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub first_min_seconds: f32,
    pub first_max_seconds: f32,
    pub min_interval_seconds: f32,
    pub max_interval_seconds: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub despawn_distance: f32,
    pub gaze_seconds: f32,
    pub jumpscares_enabled: bool,
    pub jumpscare_min_seconds: f32,
    pub jumpscare_max_seconds: f32,
    pub jumpscare_duration: f32,
    pub show_status: bool,
}
impl Default for Config {
    fn default() -> Self {
        serde_json::from_str(include_str!("../default.settings.json")).unwrap()
    }
}
impl Config {
    pub fn parse(json: &str) -> Result<Self, String> {
        if json.trim().is_empty() {
            return Ok(Self::default());
        }
        let mut base = serde_json::to_value(Self::default()).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let values = value.as_object_mut().ok_or("settings must be an object")?;
        // Retire the previous timeout without making older companions fail to load.
        for key in ["lifetime_seconds", "gaze_degrees"] {
            if let Some(old) = values.remove(key)
                && !old.is_number()
            {
                return Err("legacy setting must be numeric".into());
            }
        }
        for (key, value) in values {
            base[key] = value.clone();
        }
        let config: Self = serde_json::from_value(base).map_err(|e| e.to_string())?;
        for (min, max, low, high) in [
            (
                config.first_min_seconds,
                config.first_max_seconds,
                0.1,
                3600.,
            ),
            (
                config.min_interval_seconds,
                config.max_interval_seconds,
                0.1,
                3600.,
            ),
            (config.min_distance, config.max_distance, 4., 14.),
            (
                config.jumpscare_min_seconds,
                config.jumpscare_max_seconds,
                0.2,
                3600.,
            ),
        ] {
            if !min.is_finite() || !max.is_finite() || min < low || max > high || min > max {
                return Err("invalid interval or distance".into());
            }
        }
        for (value, low, high) in [
            (config.despawn_distance, config.max_distance + 1., 40.),
            (config.gaze_seconds, 0., 1.),
            (config.jumpscare_duration, 0.1, 2.),
        ] {
            if !value.is_finite() || !(low..=high).contains(&value) {
                return Err("invalid distance, gaze or jumpscare duration".into());
            }
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_merge_and_reject_invalid_settings() {
        assert_eq!(
            Config::parse(r#"{"enabled":false}"#).unwrap().max_distance,
            Config::default().max_distance
        );
        for input in [
            "null",
            r#"{"unknown":1}"#,
            r#"{"min_distance":15}"#,
            r#"{"first_min_seconds":20,"first_max_seconds":10}"#,
            r#"{"gaze_seconds":2}"#,
            r#"{"jumpscare_min_seconds":0.1}"#,
            r#"{"jumpscare_duration":0}"#,
            r#"{"jumpscare_min_seconds":20,"jumpscare_max_seconds":10}"#,
        ] {
            assert!(Config::parse(input).is_err());
        }
    }
    #[test]
    fn legacy_timeout_is_ignored_and_distance_must_exceed_spawn_radius() {
        let config = Config::parse(r#"{"lifetime_seconds":0.2}"#).unwrap();
        assert_eq!(config.despawn_distance, Config::default().despawn_distance);
        assert!(Config::parse(r#"{"despawn_distance":10}"#).is_err());
    }
}
