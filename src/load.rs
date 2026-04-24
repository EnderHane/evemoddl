use std::{
    collections::{
        HashMap,
        HashSet,
    },
    fs,
    path::{
        Path,
        PathBuf,
    },
};

use anyhow::{
    Context,
    Result,
    bail,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    mod_id::resolve_mod_ids,
    models::{
        ModState,
        is_ignored_dependency,
    },
};

#[derive(Serialize, Deserialize, Default)]
struct Files {
    mods: HashMap<String, ModState>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DependencyEntry {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Version")]
    _version: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModEntry {
    #[serde(rename = "Dependencies")]
    dependencies: Vec<DependencyEntry>,
    #[serde(rename = "URL")]
    _url: String,
}

pub fn run(dir: PathBuf, requested_mods: Vec<String>) -> Result<()> {
    let files_path = dir.join(".evemoddl").join("files.toml");
    let graph_path = dir.join(".evemoddl").join("mod_dependency_graph.yaml");
    let files_dir = dir.join(".evemoddl").join("files");

    if requested_mods.is_empty() {
        bail!("No mod IDs provided.");
    }

    let mut current_state: HashMap<String, ModState> = if files_path.exists() {
        let content = fs::read_to_string(&files_path).context("Failed to read files.toml")?;
        toml::from_str::<Files>(&content)
            .context("Failed to parse files.toml")?
            .mods
    } else {
        bail!("No pulled mods found. Run `pull` first.");
    };

    let graph_content = fs::read_to_string(&graph_path)
        .with_context(|| format!("Failed to read mod dependency graph at {:?}", graph_path))?;
    let graph: HashMap<String, ModEntry> = serde_yaml::from_str(&graph_content)
        .context("Failed to parse mod_dependency_graph.yaml")?;

    let requested_mods = resolve_mod_ids(&requested_mods, current_state.keys())?;

    let mut target_mods = HashSet::<String>::new();
    let mut queue = requested_mods;

    while let Some(mod_id) = queue.pop() {
        if target_mods.insert(mod_id.clone())
            && let Some(entry) = graph.get(&mod_id)
        {
            for dep in &entry.dependencies {
                if !is_ignored_dependency(&dep.name) {
                    queue.push(dep.name.clone());
                }
            }
        }
    }

    let missing_mods: Vec<String> = target_mods
        .iter()
        .filter(|mod_id| !is_pulled(&current_state, &files_dir, mod_id))
        .cloned()
        .collect();

    if !missing_mods.is_empty() {
        bail!(
            "Cannot load because these required mods are not pulled: {}. Run `pull {}` first.",
            missing_mods.join(", "),
            missing_mods.join(" ")
        );
    }

    let mut loaded_mods: Vec<String> = target_mods.into_iter().collect();
    loaded_mods.sort();

    for mod_id in &loaded_mods {
        let source_path = files_dir.join(format!("{}.zip", mod_id));
        let target_path = dir.join(format!("{}.zip", mod_id));

        if target_path.exists() {
            if let Some(state) = current_state.get_mut(mod_id) {
                state.loaded = true;
            }
            println!("{} already exists in the mods directory, skipping.", mod_id);
            continue;
        }

        fs::hard_link(&source_path, &target_path).with_context(|| {
            format!(
                "Failed to create hard link for {} from {:?} to {:?}",
                mod_id, source_path, target_path
            )
        })?;

        if let Some(state) = current_state.get_mut(mod_id) {
            state.loaded = true;
        }

        println!("Loaded {}.", mod_id);
    }

    let files_wrapper = Files {
        mods: current_state,
    };
    let toml_content = toml::to_string(&files_wrapper).context("Failed to serialize files.toml")?;
    fs::write(&files_path, toml_content).context("Failed to write files.toml")?;

    println!("Load operation completed successfully.");
    Ok(())
}

fn is_pulled(current_state: &HashMap<String, ModState>, files_dir: &Path, mod_id: &str) -> bool {
    current_state.contains_key(mod_id) && files_dir.join(format!("{}.zip", mod_id)).exists()
}
