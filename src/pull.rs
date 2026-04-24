use std::{
    collections::{
        HashMap,
        HashSet,
    },
    fs,
    path::PathBuf,
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
        ModInfo,
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
    #[serde(rename = "OptionalDependencies")]
    optional_deps: Vec<DependencyEntry>,
    #[serde(rename = "Dependencies")]
    dependencies: Vec<DependencyEntry>,
    #[serde(rename = "URL")]
    _url: String,
}

pub async fn run(dir: PathBuf, requested_mods: Vec<String>, mirror: String) -> Result<()> {
    let files_path = dir.join(".evemoddl").join("files.toml");
    let graph_path = dir.join(".evemoddl").join("mod_dependency_graph.yaml");
    let update_path = dir.join(".evemoddl").join("everest_update.yaml");

    let mut current_state = if files_path.exists() {
        let content = fs::read_to_string(&files_path).context("Failed to read files.toml")?;
        toml::from_str::<Files>(&content)
            .context("Failed to parse files.toml")?
            .mods
    } else {
        HashMap::new()
    };

    let graph_content = fs::read_to_string(&graph_path)
        .with_context(|| format!("Failed to read mod dependency graph at {:?}", graph_path))?;
    let graph: HashMap<String, ModEntry> = serde_yaml::from_str(&graph_content)
        .context("Failed to parse mod_dependency_graph.yaml")?;

    let update_content = fs::read_to_string(&update_path)
        .with_context(|| format!("Failed to read update list at {:?}", update_path))?;
    let update_list: HashMap<String, ModInfo> =
        serde_yaml::from_str(&update_content).context("Failed to parse everest_update.yaml")?;

    let requested_mods = resolve_mod_ids(&requested_mods, update_list.keys())?;

    let mut explicit_mods = HashSet::<String>::new();
    for (mod_id, state) in &current_state {
        if state.is_explicit {
            explicit_mods.insert(mod_id.clone());
        }
    }
    for mod_id in &requested_mods {
        explicit_mods.insert(mod_id.clone());
    }

    let mut target_mods = HashSet::<String>::new();
    let mut queue: Vec<String> = explicit_mods.into_iter().collect();

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

    let files_dir = dir.join(".evemoddl").join("files");
    if !files_dir.exists() {
        fs::create_dir_all(&files_dir).context("Failed to create files directory")?;
    }

    let mirror_prefix = mirror.trim_end_matches('/');

    for mod_id in target_mods {
        let info = match update_list.get(&mod_id) {
            Some(i) => i,
            None => {
                println!(
                    "Warning: Mod {} not found in update list, skipping.",
                    mod_id
                );
                continue;
            }
        };

        let current_version = current_state.get(&mod_id).map(|s| s.version.as_str());
        if current_version == Some(&info.version) {
            println!(
                "{} is already up-to-date (version {}).",
                mod_id, info.version
            );
            if requested_mods.contains(&mod_id)
                && let Some(state) = current_state.get_mut(&mod_id)
                && !state.is_explicit
            {
                state.is_explicit = true;
                save_state(&files_path, &current_state)?;
                println!("Marked {} as explicit.", mod_id);
            }
            continue;
        }

        match current_version {
            Some(version) => println!(
                "Updating {} from version {} to {}...",
                mod_id, version, info.version
            ),
            None => println!("Downloading {} fresh (version {})...", mod_id, info.version),
        }

        if let Some(entry) = graph.get(&mod_id)
            && !entry.optional_deps.is_empty()
        {
            let names: Vec<String> = entry.optional_deps.iter().map(|d| d.name.clone()).collect();
            println!("  Optional dependencies: {}", names.join(", "));
        }

        let file_id = match info.game_banana_file_id {
            Some(id) => id,
            None => {
                println!("Warning: No GameBananaFileId for {}, skipping.", mod_id);
                continue;
            }
        };

        if info.xxhash.is_empty() {
            println!("Warning: No xxHash for {}, skipping.", mod_id);
            continue;
        }

        let download_url = format!("{}/{}", mirror_prefix, file_id);
        let dest_path = files_dir.join(format!("{}.zip", mod_id));
        let temp_path = files_dir.join(format!("{}.zip.tmp", mod_id));

        let _ = fs::remove_file(&temp_path);

        if let Err(e) = crate::download::download_file(&download_url, &temp_path).await {
            println!("Failed to download {}: {}", mod_id, e);
            let _ = fs::remove_file(&temp_path);
            continue;
        }

        if let Err(e) = crate::download::verify_xxhash(&temp_path, &info.xxhash) {
            println!("Failed to verify {}: {}", mod_id, e);
            let _ = fs::remove_file(&temp_path);
            continue;
        }

        fs::rename(&temp_path, &dest_path)
            .with_context(|| format!("Failed to replace downloaded file for {}", mod_id))?;

        current_state.insert(
            mod_id.clone(),
            ModState {
                version: info.version.clone(),
                is_explicit: requested_mods.contains(&mod_id)
                    || current_state.get(&mod_id).is_some_and(|s| s.is_explicit),
                loaded: current_state.get(&mod_id).is_some_and(|s| s.loaded),
            },
        );

        save_state(&files_path, &current_state)?;

        println!("Successfully downloaded and updated {}.", mod_id);
    }

    Ok(())
}

fn save_state(
    files_path: &std::path::Path,
    current_state: &HashMap<String, ModState>,
) -> Result<()> {
    let files_wrapper = Files {
        mods: current_state.clone(),
    };
    let toml_content = toml::to_string(&files_wrapper).context("Failed to serialize files.toml")?;
    fs::write(files_path, toml_content).context("Failed to write files.toml")?;
    Ok(())
}
