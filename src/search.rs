use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
};

use anyhow::{
    Context,
    Result,
};

use crate::models::ModInfo;

pub fn run(dir: PathBuf, query: String) -> Result<()> {
    let update_path = dir.join(".evemoddl").join("everest_update.yaml");
    let content = fs::read_to_string(&update_path)
        .with_context(|| format!("Failed to read update file at {:?}", update_path))?;
    let mods: HashMap<String, ModInfo> =
        serde_yaml::from_str(&content).context("Failed to parse everest_update.yaml")?;

    let query_lower = query.to_lowercase();
    let matches: Vec<(&String, &ModInfo)> = mods
        .iter()
        .filter(|(mod_id, _)| mod_id.to_lowercase().contains(&query_lower))
        .collect();

    if !matches.is_empty() {
        for (mod_id, info) in matches {
            println!(
                "{} | {} | https://gamebanana.com/mods/{}",
                mod_id, info.version, info.game_banana_id
            );
        }
    } else {
        let mut similar: Vec<(f64, &String)> = mods
            .keys()
            .map(|mod_id| (strsim::normalized_levenshtein(&query, mod_id), mod_id))
            .collect();

        similar.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        println!("No exact match found. Did you mean:");
        for (_, mod_id) in similar.into_iter().take(3) {
            println!("{}", mod_id);
        }
    }

    Ok(())
}
