//! Export the exact generated mod mesh for external visual inspection.
#[path = "../src/logic.rs"]
#[allow(dead_code)]
mod logic;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pieces=model::build().into_iter().map(|piece|{
        let mod_api::drawing::api::GeometrySpec::Triangles(mesh)=piece else{unreachable!()};
        serde_json::json!({"vertices":mesh.vertices.iter().map(|v|serde_json::json!({"position":v.position,"uv":v.uv,"color":v.color})).collect::<Vec<_>>(),"indices":mesh.indices})
    }).collect::<Vec<_>>();
    let path = std::env::args().nth(1).ok_or("supply output JSON path")?;
    std::fs::write(path, serde_json::to_vec(&pieces)?)?;
    Ok(())
}
