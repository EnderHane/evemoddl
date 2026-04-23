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

use crate::models::{
    ModInfo,
    ModState,
};

#[derive(Serialize, Deserialize, Default)]
struct Files {
    mods: HashMap<String, ModState>,
}

pub async fn run(dir: PathBuf, requested_mods: Vec<String>, mirror: String) -> Result<()> {
    let files_path = dir.join(".evemoddl").join("files.toml");
    let graph_path = dir.join(".evemoddl").join("mod_dependency_graph.yaml");
    let update_path = dir.join(".evemoddl").join("everest_update.yaml");

    // 1. Load current state
    let mut current_state = if files_path.exists() {
        let content = fs::read_to_string(&files_path).context("Failed to read files.toml")?;
        toml::from_str::<Files>(&content)
            .context("Failed to parse files.toml")?
            .mods
    } else {
        HashMap::new()
    };

    // 2. Determine explicit mods
    let mut explicit_mods = HashSet::<String>::new();
    for (mod_id, state) in &current_state {
        if state.is_explicit {
            explicit_mods.insert(mod_id.clone());
        }
    }
    for mod_id in &requested_mods {
        explicit_mods.insert(mod_id.clone());
    }

    // 3. Resolve dependencies
    let graph_content = fs::read_to_string(&graph_path)
        .with_context(|| format!("Failed to read mod dependency graph at {:?}", graph_path))?;
    let graph: HashMap<String, Vec<String>> = serde_yaml::from_str(&graph_content)
        .context("Failed to parse mod_dependency_graph.yaml")?;

    let mut target_mods = HashSet::<String>::new();
    let mut queue: Vec<String> = explicit_mods.into_iter().collect();

    while let Some(mod_id) = queue.pop() {
        if target_mods.insert(mod_id.clone())
            && let Some(deps) = graph.get(&mod_id)
        {
            for dep in deps {
                queue.push(dep.clone());
            }
        }
    }

    // 4. Get latest versions from update list
    let update_content = fs::read_to_string(&update_path)
        .with_context(|| format!("Failed to read update list at {:?}", update_path))?;
    let update_list: HashMap<String, ModInfo> =
        serde_yaml::from_str(&update_content).context("Failed to parse everest_update.yaml")?;

    // 5. Download needed mods
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
            continue;
        }

        println!("Downloading {} (version {})...", mod_id, info.version);

        let file_id = match info.game_banana_file_id {
            Some(id) => id,
            None => {
                println!("Warning: No GameBananaFileId for {}, skipping.", mod_id);
                continue;
            }
        };

        let download_url = format!("{}/{}", mirror_prefix, file_id);
        let dest_path = files_dir.join(format!("{}.zip", mod_id));

        if let Err(e) = download_file(&download_url, &dest_path).await {
            println!("Failed to download {}: {}", mod_id, e);
            continue;
        }

        // Update state
        current_state.insert(
            mod_id.clone(),
            ModState {
                version: info.version.clone(),
                is_explicit: requested_mods.contains(&mod_id)
                    || current_state.get(&mod_id).is_some_and(|s| s.is_explicit),
            },
        );

        // Save state incrementally
        let files_wrapper = Files {
            mods: current_state.clone(),
        };
        let toml_content =
            toml::to_string(&files_wrapper).context("Failed to serialize files.toml")?;
        fs::write(&files_path, toml_content).context("Failed to write files.toml")?;

        println!("Successfully downloaded and updated {}.", mod_id);
    }

    Ok(())
}

async fn download_file(url: &str, dest: &Path) -> Result<()> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to send request to {}", url))?
        .bytes()
        .await
        .with_context(|| format!("Failed to read bytes from {}", url))?;

    fs::write(dest, response).context("Failed to write downloaded file to disk")?;
    Ok(())
}
