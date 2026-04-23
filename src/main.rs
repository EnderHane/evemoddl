use std::path::PathBuf;

use anyhow::{
    Context,
    Result,
};
use clap::{
    Parser,
    Subcommand,
};

mod config;
mod models;
mod pull;
mod search;
mod update;

use config::Config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Runtime specifying working directory
    #[arg(short = 'C')]
    working_dir: Option<PathBuf>,

    /// Mirror for updating data files
    #[arg(short = 'm', long)]
    mirror: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Save mods directory to the conventional config file
    SetModsDir {
        /// The directory to set as mods directory
        directory: PathBuf,
    },
    /// Update mod dependency graph and update list
    Update,
    /// Search for mods in the update list
    Search {
        /// ModID or substring to search for
        query: String,
    },
    /// Pull mods and their dependencies
    Pull {
        /// Mod IDs to pull
        #[arg(value_name = "MODID")]
        mod_ids: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load();

    // Resolve working directory
    let active_dir = cli.working_dir.or(config.mods_dir.clone());

    match cli.command {
        Commands::SetModsDir { directory } => {
            config.mods_dir = Some(directory.clone());
            config
                .save()
                .context("Failed to save mods directory to config")?;
            println!("Mods directory set to: {:?}", directory);
        }
        Commands::Update => {
            let dir = active_dir.ok_or_else(|| {
                anyhow::anyhow!("Mods directory not set. Use 'set-mods-dir' or -C to specify one.")
            })?;

            let mirror = cli
                .mirror
                .or_else(|| std::env::var("EVEMODDL_UPDATE_MIRROR").ok())
                .or_else(|| config.update_mirror.clone())
                .unwrap_or_else(|| "https://everestapi.github.io/updatermirror/".to_string());

            update::run(dir, mirror).await?;
        }
        Commands::Search { query } => {
            let dir = active_dir.ok_or_else(|| {
                anyhow::anyhow!("Mods directory not set. Use 'set-mods-dir' or -C to specify one.")
            })?;

            search::run(dir, query)?;
        }
        Commands::Pull { mod_ids } => {
            let dir = active_dir.ok_or_else(|| {
                anyhow::anyhow!("Mods directory not set. Use 'set-mods-dir' or -C to specify one.")
            })?;

            let mirror = cli
                .mirror
                .or_else(|| std::env::var("EVEMODDL_GAMEBANANA_MIRROR").ok())
                .or_else(|| config.gamebanana_mirror.clone())
                .unwrap_or_else(|| "https://gamebanana.com/mmdl".to_string());

            pull::run(dir, mod_ids, mirror).await?;
        }
    }

    Ok(())
}

// I need to add .context for SetModsDir since I removed anyhow::Context import.
// Wait, I'll just add the import back or use a different way.
