use std::path::PathBuf;

use anyhow::Result;

pub async fn run(dir: PathBuf, mirror: String) -> Result<()> {
    println!("Updating in directory: {:?}", dir);
    println!("Using mirror: {}", mirror);

    let modgraph_dest = dir.join(".evemoddl").join("mod_dependency_graph.yaml");
    let modupdater_dest = dir.join(".evemoddl").join("everest_update.yaml");

    let modgraph_url = format!("{}/mod_dependency_graph.yaml", mirror.trim_end_matches('/'));
    crate::download::download_file(&modgraph_url, &modgraph_dest)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to download mod dependency graph from {}: {}",
                modgraph_url,
                e
            )
        })?;
    println!("Downloaded mod_dependency_graph.yaml");

    let modupdater_url = format!("{}/everest_update.yaml", mirror.trim_end_matches('/'));
    crate::download::download_file(&modupdater_url, &modupdater_dest)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to download everest update list from {}: {}",
                modupdater_url,
                e
            )
        })?;
    println!("Downloaded everest_update.yaml");

    Ok(())
}
