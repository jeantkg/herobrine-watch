//! Compare a new component's imports against an already working component.
use std::collections::HashSet;
use wit_component::{DecodedWasm, decode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: compat WORKING.wasm CANDIDATE.wasm".into());
    }
    let DecodedWasm::Component(mut baseline, old_world) = decode(&std::fs::read(&args[0])?)? else {
        return Err("working file is not a component".into());
    };
    let DecodedWasm::Component(mut candidate, new_world) = decode(&std::fs::read(&args[1])?)?
    else {
        return Err("candidate file is not a component".into());
    };
    let old: HashSet<_> = baseline.worlds[old_world]
        .imports
        .keys()
        .map(|key| baseline.name_world_key(key))
        .collect();
    for key in candidate.worlds[new_world].imports.keys() {
        let name = candidate.name_world_key(key);
        if !old.contains(&name) {
            return Err(format!("new import not present in working component: {name}").into());
        }
    }
    // Decoding gives both adapters a synthetic root world. Their import counts may differ;
    // compare the shared API packages without treating those adapters as the same world.
    if let Some(package) = candidate.worlds[new_world].package {
        let old = candidate.packages[package].name.clone();
        let mut name = old.clone();
        name.name.push_str("-candidate");
        candidate.package_names.shift_remove(&old);
        candidate.packages[package].name = name.clone();
        candidate.package_names.insert(name, package);
    }
    // Shared package/interface/type definitions must agree, not merely have matching names.
    baseline.merge(candidate)?;
    println!(
        "PASS: candidate imports are a subset of the working component; shared WIT definitions agree."
    );
    Ok(())
}
