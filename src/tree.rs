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

pub fn run(dir: PathBuf, mod_id: Option<String>, loaded_only: bool) -> Result<()> {
    let graph = load_graph(&dir)?;

    if let Some(mod_id) = mod_id {
        let mod_id = resolve_mod_ids(&[mod_id], graph.keys())?.remove(0);
        print_tree(&mod_id, &graph, &|_| true);
        return Ok(());
    }

    let state = load_state(&dir)?;
    let mut roots: Vec<String> = state
        .iter()
        .filter(|(_, mod_state)| mod_state.is_explicit && (!loaded_only || mod_state.loaded))
        .map(|(mod_id, _)| mod_id.clone())
        .collect();
    roots.sort();

    if roots.is_empty() {
        if loaded_only {
            println!("No loaded explicit mods found.");
        } else {
            println!("No explicit mods found.");
        }
        return Ok(());
    }

    let include_node = |name: &str| {
        state
            .get(name)
            .is_some_and(|mod_state| !loaded_only || mod_state.loaded)
    };

    print_forest(&roots, &graph, &include_node);
    Ok(())
}

fn load_graph(dir: &Path) -> Result<HashMap<String, ModEntry>> {
    let graph_path = dir.join(".evemoddl").join("mod_dependency_graph.yaml");
    let graph_content = fs::read_to_string(&graph_path)
        .with_context(|| format!("Failed to read mod dependency graph at {:?}", graph_path))?;
    serde_yaml::from_str(&graph_content).context("Failed to parse mod_dependency_graph.yaml")
}

fn load_state(dir: &Path) -> Result<HashMap<String, ModState>> {
    let files_path = dir.join(".evemoddl").join("files.toml");
    let content = fs::read_to_string(&files_path)
        .with_context(|| format!("Failed to read files.toml at {:?}", files_path))?;
    Ok(toml::from_str::<Files>(&content)
        .context("Failed to parse files.toml")?
        .mods)
}

fn print_forest<F>(roots: &[String], graph: &HashMap<String, ModEntry>, include_node: &F)
where
    F: Fn(&str) -> bool,
{
    for root in roots {
        println!("{}", root);
        let mut active_path = HashSet::from([root.clone()]);
        let children = filtered_dependencies(root, graph, include_node);
        render_children(
            &children,
            String::new(),
            graph,
            include_node,
            &mut active_path,
        );
    }
}

fn print_tree<F>(root: &str, graph: &HashMap<String, ModEntry>, include_node: &F)
where
    F: Fn(&str) -> bool,
{
    println!("{}", root);
    let mut active_path = HashSet::from([root.to_string()]);
    let children = filtered_dependencies(root, graph, include_node);
    render_children(
        &children,
        String::new(),
        graph,
        include_node,
        &mut active_path,
    );
}

fn render_children<F>(
    children: &[String],
    prefix: String,
    graph: &HashMap<String, ModEntry>,
    include_node: &F,
    active_path: &mut HashSet<String>,
) where
    F: Fn(&str) -> bool,
{
    for (index, child) in children.iter().enumerate() {
        let is_last = index + 1 == children.len();
        let branch = if is_last { "└── " } else { "├── " };
        let next_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

        if active_path.contains(child) {
            println!("{}{}{} (cycle)", prefix, branch, child);
            continue;
        }

        println!("{}{}{}", prefix, branch, child);
        active_path.insert(child.clone());
        let grandchildren = filtered_dependencies(child, graph, include_node);
        render_children(
            &grandchildren,
            next_prefix,
            graph,
            include_node,
            active_path,
        );
        active_path.remove(child);
    }
}

fn filtered_dependencies<F>(
    mod_id: &str,
    graph: &HashMap<String, ModEntry>,
    include_node: &F,
) -> Vec<String>
where
    F: Fn(&str) -> bool,
{
    let Some(entry) = graph.get(mod_id) else {
        return Vec::new();
    };

    let mut dependencies: Vec<String> = entry
        .dependencies
        .iter()
        .filter(|dep| !is_ignored_dependency(&dep.name))
        .map(|dep| dep.name.clone())
        .filter(|dep| include_node(dep))
        .collect();
    dependencies.sort();
    dependencies
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        DependencyEntry,
        ModEntry,
        filtered_dependencies,
    };
    use crate::models::ModState;

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
    fn filters_core_dependencies() {
        let graph = HashMap::from([(
            "Root".to_string(),
            mod_entry(&["Celeste", "Everest", "EverestCore", "RealDep"]),
        )]);

        let dependencies = filtered_dependencies("Root", &graph, &|_| true);

        assert_eq!(dependencies, vec!["RealDep".to_string()]);
    }

    #[test]
    fn filters_local_tree_by_loaded_state() {
        let graph = HashMap::from([("Root".to_string(), mod_entry(&["LoadedDep", "UnloadedDep"]))]);
        let state = HashMap::from([
            (
                "LoadedDep".to_string(),
                ModState {
                    version: "1.0.0".to_string(),
                    is_explicit: false,
                    loaded: true,
                },
            ),
            (
                "UnloadedDep".to_string(),
                ModState {
                    version: "1.0.0".to_string(),
                    is_explicit: false,
                    loaded: false,
                },
            ),
        ]);

        let dependencies = filtered_dependencies("Root", &graph, &|name| {
            state.get(name).is_some_and(|s| s.loaded)
        });

        assert_eq!(dependencies, vec!["LoadedDep".to_string()]);
    }
}
