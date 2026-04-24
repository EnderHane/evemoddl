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

pub fn run(dir: PathBuf, mod_ids: Vec<String>) -> Result<()> {
    if mod_ids.is_empty() {
        bail!("No mod IDs provided.");
    }

    let files_path = dir.join(".evemoddl").join("files.toml");
    let mut current_state = load_state(&files_path)?;
    let mod_ids = resolve_mod_ids(&mod_ids, current_state.keys())?;
    let unloaded_count = unload_from_state(&dir, &mut current_state, &mod_ids)?;

    save_state(&files_path, current_state)?;

    if unloaded_count == 0 {
        println!("No mods were unloaded.");
    } else {
        println!("Unload operation completed successfully.");
    }

    Ok(())
}

pub fn unload_from_state(
    dir: &Path,
    current_state: &mut HashMap<String, ModState>,
    mod_ids: &[String],
) -> Result<usize> {
    if mod_ids.is_empty() {
        bail!("No mod IDs provided.");
    }

    let mut requested_unloads = HashSet::new();
    for mod_id in mod_ids {
        match current_state.get(mod_id) {
            Some(state) if !state.is_explicit => {
                println!(
                    "Warning: {} is a dependency and cannot be unloaded directly. Skipping.",
                    mod_id
                );
            }
            Some(state) if !state.loaded => {
                println!("Warning: {} is not loaded. Skipping.", mod_id);
            }
            Some(_) => {
                requested_unloads.insert(mod_id.clone());
            }
            None => {
                println!("Warning: {} is not installed. Skipping.", mod_id);
            }
        }
    }

    if requested_unloads.is_empty() {
        return Ok(0);
    }

    let graph = load_graph(dir)?;
    let mods_to_keep_loaded = collect_needed_mods(
        current_state
            .iter()
            .filter(|(mod_id, state)| {
                state.loaded && state.is_explicit && !requested_unloads.contains(mod_id.as_str())
            })
            .map(|(mod_id, _)| mod_id.clone())
            .collect(),
        &graph,
    );

    let mut mods_to_unload: Vec<String> = current_state
        .iter()
        .filter(|(mod_id, state)| state.loaded && !mods_to_keep_loaded.contains(mod_id.as_str()))
        .map(|(mod_id, _)| mod_id.clone())
        .collect();
    mods_to_unload.sort();

    for mod_id in &mods_to_unload {
        remove_loaded_link(dir, mod_id)?;
        if let Some(state) = current_state.get_mut(mod_id) {
            state.loaded = false;
        }
        println!("Unloaded {}.", mod_id);
    }

    Ok(mods_to_unload.len())
}

fn load_state(files_path: &Path) -> Result<HashMap<String, ModState>> {
    if !files_path.exists() {
        bail!("No pulled mods found. Run `pull` first.");
    }

    let content = fs::read_to_string(files_path).context("Failed to read files.toml")?;
    Ok(toml::from_str::<Files>(&content)
        .context("Failed to parse files.toml")?
        .mods)
}

fn save_state(files_path: &Path, current_state: HashMap<String, ModState>) -> Result<()> {
    let files_wrapper = Files {
        mods: current_state,
    };
    let toml_content = toml::to_string(&files_wrapper).context("Failed to serialize files.toml")?;
    fs::write(files_path, toml_content).context("Failed to write files.toml")?;
    Ok(())
}

fn load_graph(dir: &Path) -> Result<HashMap<String, ModEntry>> {
    let graph_path = dir.join(".evemoddl").join("mod_dependency_graph.yaml");
    let graph_content = fs::read_to_string(&graph_path)
        .with_context(|| format!("Failed to read mod dependency graph at {:?}", graph_path))?;
    serde_yaml::from_str(&graph_content).context("Failed to parse mod_dependency_graph.yaml")
}

fn collect_needed_mods(
    explicit_mods: Vec<String>,
    graph: &HashMap<String, ModEntry>,
) -> HashSet<String> {
    let mut needed_mods = HashSet::new();
    let mut queue = explicit_mods;

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

    needed_mods
}

fn remove_loaded_link(dir: &Path, mod_id: &str) -> Result<()> {
    let link_path = dir.join(format!("{}.zip", mod_id));
    if link_path.exists() {
        fs::remove_file(&link_path)
            .with_context(|| format!("Failed to remove loaded file for {}", mod_id))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        DependencyEntry,
        ModEntry,
        collect_needed_mods,
    };

    fn mod_entry(dependencies: &[&str]) -> ModEntry {
        ModEntry {
            dependencies: dependencies
                .iter()
                .map(|name| DependencyEntry {
                    name: (*name).to_string(),
                    _version: "1.0.0".to_string(),
                })
                .collect(),
            _url: String::new(),
        }
    }

    #[test]
    fn keeps_transitive_dependencies_for_remaining_loaded_explicit_mods() {
        let graph = HashMap::from([
            ("A".to_string(), mod_entry(&["Shared", "OnlyA"])),
            ("B".to_string(), mod_entry(&["Shared", "OnlyB"])),
            ("Shared".to_string(), mod_entry(&[])),
            ("OnlyA".to_string(), mod_entry(&[])),
            ("OnlyB".to_string(), mod_entry(&[])),
        ]);

        let needed = collect_needed_mods(vec!["B".to_string()], &graph);

        assert!(needed.contains("B"));
        assert!(needed.contains("Shared"));
        assert!(needed.contains("OnlyB"));
        assert!(!needed.contains("A"));
        assert!(!needed.contains("OnlyA"));
    }

    #[test]
    fn skips_core_mod_dependencies() {
        let graph = HashMap::from([(
            "A".to_string(),
            mod_entry(&["Celeste", "Everest", "EverestCore", "RealDep"]),
        )]);

        let needed = collect_needed_mods(vec!["A".to_string()], &graph);

        assert!(needed.contains("A"));
        assert!(needed.contains("RealDep"));
        assert!(!needed.contains("Celeste"));
        assert!(!needed.contains("Everest"));
        assert!(!needed.contains("EverestCore"));
    }
}
