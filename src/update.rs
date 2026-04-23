use std::{
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

pub async fn run(dir: PathBuf, mirror: String) -> Result<()> {
    println!("Updating in directory: {:?}", dir);
    println!("Using mirror: {}", mirror);

    let modgraph_dest = dir.join(".evemoddl").join("mod_dependency_graph.yaml");
    let modupdater_dest = dir.join(".evemoddl").join("everest_update.yaml");

    let modgraph_url = format!("{}/mod_dependency_graph.yaml", mirror.trim_end_matches('/'));
    download_file(&modgraph_url, &modgraph_dest)
        .await
        .with_context(|| {
            format!(
                "Failed to download mod dependency graph from {}",
                modgraph_url
            )
        })?;
    println!("Downloaded mod_dependency_graph.yaml");

    let modupdater_url = format!("{}/everest_update.yaml", mirror.trim_end_matches('/'));
    download_file(&modupdater_url, &modupdater_dest)
        .await
        .with_context(|| {
            format!(
                "Failed to download everest update list from {}",
                modupdater_url
            )
        })?;
    println!("Downloaded everest_update.yaml");

    Ok(())
}

async fn download_file(url: &str, dest: &Path) -> Result<()> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to send request to {}", url))?
        .bytes()
        .await
        .with_context(|| format!("Failed to read bytes from {}", url))?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).context("Failed to create directory for downloaded file")?;
    }
    fs::write(dest, response).context("Failed to write downloaded file to disk")?;
    Ok(())
}
