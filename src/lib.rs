mod config;
mod jumpscare;
mod logic;
mod model;
mod runtime;
mod terrain;
use mod_api::PlayerMod;
use std::cell::RefCell;
thread_local! { static RUNTIME:RefCell<runtime::Runtime>=RefCell::new(runtime::Runtime::default()); }
struct HerobrineWatch;
impl PlayerMod for HerobrineWatch {
    fn init() {
        RUNTIME.with(|value| value.borrow_mut().init());
    }
    fn frame() {
        RUNTIME.with(|value| value.borrow_mut().frame());
    }
}
mod_api::export_player_mod!(HerobrineWatch);

#[cfg(test)]
mod tests {
    #[test]
    fn permissions_are_limited_to_local_observations_settings_and_drawing() {
        let grants: serde_json::Value =
            serde_json::from_str(include_str!("../grants.json")).unwrap();
        for name in [
            "player_state",
            "terrain",
            "world_draw",
            "settings",
            "hud",
            "render_post",
        ] {
            assert_eq!(grants[name], true);
        }
        assert_eq!(grants.as_object().unwrap().len(), 6);
    }
}
