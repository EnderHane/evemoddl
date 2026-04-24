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
    unload,
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

pub fn run(dir: PathBuf, mod_ids: Vec<String>) -> Result<()> {
    let files_path = dir.join(".evemoddl").join("files.toml");
    let graph_path = dir.join(".evemoddl").join("mod_dependency_graph.yaml");
    let files_dir = dir.join(".evemoddl").join("files");

    let mut current_state: HashMap<String, ModState> = if files_path.exists() {
        let content = fs::read_to_string(&files_path).context("Failed to read files.toml")?;
        toml::from_str::<Files>(&content)
            .context("Failed to parse files.toml")?
            .mods
    } else {
        println!("No mods installed.");
        return Ok(());
    };

    let mod_ids = resolve_mod_ids(&mod_ids, current_state.keys())?;

    unload::unload_from_state(&dir, &mut current_state, &mod_ids)?;

    for mod_id in &mod_ids {
        match current_state.get(mod_id) {
            Some(state) => {
                if !state.is_explicit {
                    println!(
                        "Warning: {} is a dependency and cannot be removed directly. Skipping.",
                        mod_id
                    );
                    continue;
                }
                remove_loaded_link(&dir, mod_id, state.loaded)?;
                current_state.remove(mod_id);
                let file_path = files_dir.join(format!("{}.zip", mod_id));
                if file_path.exists() {
                    fs::remove_file(&file_path)
                        .with_context(|| format!("Failed to remove file for {}", mod_id))?;
                    println!("Deleted file for {}.", mod_id);
                }
                println!("Removing {} from explicit mods...", mod_id);
            }
            None => {
                println!("Warning: {} is not installed. Skipping.", mod_id);
            }
        }
    }

    let graph_content = fs::read_to_string(&graph_path)
        .with_context(|| format!("Failed to read mod dependency graph at {:?}", graph_path))?;
    let graph: HashMap<String, ModEntry> = serde_yaml::from_str(&graph_content)
        .context("Failed to parse mod_dependency_graph.yaml")?;

    let remaining_explicit: Vec<String> = current_state
        .iter()
        .filter(|(_, state)| state.is_explicit)
        .map(|(mod_id, _)| mod_id.clone())
        .collect();

    let mut needed_mods = HashSet::<String>::new();
    let mut queue = remaining_explicit;

    while let Some(mod_id) = queue.pop() {
        if needed_mods.insert(mod_id.clone())
            && let Some(entry) = graph.get(&mod_id)
        {
            for dep in &entry.dependencies {
                if !is_ignored_dependency(&dep.name) {
                    queue.push(dep.name.clone());
                }
            }
        }
    }

    let orphaned: Vec<String> = current_state
        .keys()
        .filter(|mod_id| !needed_mods.contains(mod_id.as_str()))
        .cloned()
        .collect();

    for mod_id in orphaned {
        let was_loaded = current_state.get(&mod_id).is_some_and(|state| state.loaded);
        remove_loaded_link(&dir, &mod_id, was_loaded)?;
        let file_path = files_dir.join(format!("{}.zip", mod_id));
        if file_path.exists() {
            fs::remove_file(&file_path)
                .with_context(|| format!("Failed to remove file for {}", mod_id))?;
            println!("Deleted file for {}.", mod_id);
        }
        // Remove from state
        current_state.remove(&mod_id);
        println!("Removed {} from installed mods.", mod_id);
    }

    let files_wrapper = Files {
        mods: current_state,
    };
    let toml_content = toml::to_string(&files_wrapper).context("Failed to serialize files.toml")?;
    fs::write(&files_path, toml_content).context("Failed to write files.toml")?;

    println!("Remove operation completed successfully.");
    Ok(())
}

fn remove_loaded_link(dir: &Path, mod_id: &str, was_loaded: bool) -> Result<()> {
    if !was_loaded {
        return Ok(());
    }

    let link_path = dir.join(format!("{}.zip", mod_id));
    if link_path.exists() {
        fs::remove_file(&link_path)
            .with_context(|| format!("Failed to remove loaded file for {}", mod_id))?;
        println!("Deleted loaded file for {}.", mod_id);
    }

    Ok(())
}
