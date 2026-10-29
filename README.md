# Herobrine Watch

A standalone client-only scare mod for Cinnabar's world-draw 0.2 API. A standard
classic player mesh wearing the supplied Herobrine skin appears nearby at random times, watches you,
and disappears when you look at him. It is a local depth-tested drawing, not a
server entity: no attacks, damage, collision, world edits or network commands.

## Install and play

Import `herobrine-watch.wasm` in **Mods**, enable it and allow Player state, Terrain,
World drawing, Post-processing and Settings. HUD is used only for errors. The optional
`herobrine-watch.settings.json` belongs beside the WASM. Reload after editing it.
This mod can coexist with Procedural Portals. No client rebuild is needed.

In a normal world, stay in focused gameplay near solid ground. The first spawn is
attempted after 8–18 seconds. Later attempts are 30–90 seconds apart. Herobrine
appears 8–14 blocks away, outside your central view, and turns to face your character's
position. Third-person camera movement alone does not turn him. Look
towards any part of the player model to make him vanish; wall-blocked gaze does not
count. He stays indefinitely until looked at or until the player moves more than
`despawn_distance` blocks away (32 by default). Unsuitable or unloaded terrain skips an attempt.

Pause and menus preserve a valid appearance and freeze its logic. Death, disconnect,
dimension changes and mod reload remove it. He does not follow or chase you.
Separately, his face fills the screen briefly at random intervals (60–180 gameplay
seconds, lasting 0.65 seconds by default). It uses the face from the supplied skin,
with a small zoom and a black backdrop. There is no sound or strobing. Menus cancel
an active face scare; death, disconnect and dimension changes clear it.
The supplied 64x64 skin is mapped to the classic six-part player geometry, with
support for transparent outer layers. Skin texels become coloured rectangles because
the API does not expose native player/skin creation or texture binding. This is a
static player mesh, not the native animated player renderer. White eyes are unlit
white geometry, with no bloom or sound effect. The user-supplied skin has no additional
redistribution license asserted here; the project's MIT license covers its code.

## Settings

Missing settings use `default.settings.json`. All intervals are gameplay seconds.

| Setting | Meaning |
| --- | --- |
| `enabled` | Enable the mod. |
| `first_min_seconds`, `first_max_seconds` | Random first-attempt delay. |
| `min_interval_seconds`, `max_interval_seconds` | Random subsequent attempt delay. |
| `min_distance`, `max_distance` | Spawn radius, within 4–14 blocks. |
| `despawn_distance` | Removal distance from the player's feet, greater than the spawn radius and at most 40 blocks. |
| `gaze_seconds` | Clear-view gaze duration before removal, 0–1 seconds. |
| `jumpscares_enabled` | Enable the independent face-only screen scares. |
| `jumpscare_min_seconds`, `jumpscare_max_seconds` | Random delay between face scares, 0.2–3600 gameplay seconds. |
| `jumpscare_duration` | Face display duration, 0.1–2 seconds. |
| `show_status` | Show error messages. |

Legacy `lifetime_seconds` and `gaze_degrees` keys are accepted and ignored. There is
no lifetime timer or angular-cone despawn: the camera ray must hit a player body part.

For a quick test, set both first delays to `1`, both interval delays to `5`, and
reload. Keep the normal intervals for less predictable scares.

## Windows build

Keep Cinnabar's public checkout beside this project as `../cinnabar`, with the
world-draw 0.2 SDK. Install the Rust toolchain/target in `rust-toolchain.toml`.
For a matching SDK baseline, use Cinnabar commit
[`057e4b14d3b5873259a4b6c13c277c82ee308b9a`](https://github.com/cinnabar-mc/cinnabar/commit/057e4b14d3b5873259a4b6c13c277c82ee308b9a).
From the directory where you keep both projects:

```powershell
git clone https://github.com/cinnabar-mc/cinnabar.git
git -C cinnabar checkout 057e4b14d3b5873259a4b6c13c277c82ee308b9a
git clone https://github.com/jeantkg/herobrine-watch.git
cd herobrine-watch
```

Python 3 (standard library only) verifies the embedded skin pixels against the PNG
before builds. To regenerate them after replacing the skin, run
`python tools/prepare_skin.py --write`.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\Build.ps1
# GNU fallback if the matching toolchain is installed and MSVC link.exe is unavailable:
powershell -NoProfile -ExecutionPolicy Bypass -File .\Build.ps1 -Toolchain 1.99.0-x86_64-pc-windows-gnu
```

Only the mod, component packer and tests are built. The packer/compatibility checker
were adapted from the MIT-licensed Procedural Portals project. No client or server
binary is built, modified or included.

## Limits and verification

Standing placement requires a loaded full-cube collision floor and empty headroom.
Complex partial floors, water and unloaded terrain can prevent an appearance. Gaze
uses the observed camera ray, six player boxes and loaded collision geometry;
glass blocks gaze. Long third-person offsets may exceed the query bounds and stop
gaze detection until the camera is closer. Collision queries are asynchronous.

The six-part model is genuinely three dimensional. Its immutable mesh batches share
a private depth buffer through a small, invisible mesh clip. This prevents rear
faces overpainting the face and body; ordinary un-clipped mod triangles do not write
self-depth in the current renderer. The clip is allocated once per apparition, and
model geometry is reused. No client change is required.

World depth occludes the surrounding mask. Because clip contents test their own
depth, a world surface intersecting the small mask can show an edge error of up to
roughly one block; this is not exact native-entity depth integration. Its unlit colours do not
reproduce the game's full entity lighting, skin renderer or animations. Other
camera-mod roll/tilt is not exposed by the camera-pose API and may affect gaze.
Resource admission and behaviour can be tested with
`powershell -NoProfile -ExecutionPolicy Bypass -File .\Verify-Host.ps1 -Component .\dist\herobrine-watch.wasm -Toolchain 1.99.0-x86_64-pc-windows-gnu`.
The script stages a copy with a fast-test companion under `.local`; it does not
change installed settings. Actual in-game appearance, camera modes and GPU rendering
remain separate acceptance checks.

Skin pixel coverage, classic UV mapping, body-part gaze, settings migration and
terrain safety have unit coverage. Packed native-host validation covers placement,
gaze, wall occlusion, geometry reuse, persistence, distance and world lifecycle.
It also validates the face shader, timed activation/cancellation and model clip
references. The test fixture simulates GPU clip readiness; it does not test GPU
allocation or rendering.
Native drawing admission does not render a game frame; in-game acceptance is pending.

The exact generated mesh was exported to Blockbench for front/back/side inspection
with the supplied skin. The editable project is `review/herobrine-runtime.bbmodel`.
Reproducing the no-depth-write pipeline in Blockbench shows rear skin faces covering
the front; enabling depth writes restores the correctly facing three-dimensional
appearance. This isolated reproduction is separate from live Cinnabar acceptance.
