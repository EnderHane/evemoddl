use std::path::PathBuf;

use anyhow::{
    Context,
    Result,
};
use clap::{
    Parser,
    Subcommand,
    ValueEnum,
};

mod config;
mod download;
mod load;
mod mod_id;
mod models;
mod pull;
mod remove;
mod search;
mod tree;
mod unload;
mod update;

use config::Config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Runtime specifying working directory
    #[arg(short = 'C')]
    working_dir: Option<PathBuf>,

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
    /// Show or change persisted configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Update mod dependency graph and update list
    Update {
        /// Mirror for updating data files
        #[arg(short = 'm', long)]
        mirror: Option<String>,
    },
    /// Search for mods in the update list
    Search {
        /// ModID or substring to search for
        query: String,
    },
    /// Print dependency trees
    Tree {
        /// Show the loaded tree from the active mods directory
        #[arg(long, conflicts_with = "mod_id")]
        loaded: bool,
        /// Mod ID to inspect from the dependency graph
        #[arg(value_name = "MODID")]
        mod_id: Option<String>,
    },
    /// Pull mods and their dependencies
    #[command(alias = "download", alias = "dl")]
    Pull {
        /// Mod IDs to pull
        #[arg(value_name = "MODID")]
        mod_ids: Vec<String>,
        /// Mirror for downloading mods
        #[arg(short = 'm', long)]
        mirror: Option<String>,
    },
    /// Load pulled mods into the active mods directory
    Load {
        /// Mod IDs to load
        #[arg(value_name = "MODID")]
        mod_ids: Vec<String>,
    },
    /// Unlink loaded mods not needed anymore
    Unload {
        /// Mod IDs to unload
        #[arg(value_name = "MODID")]
        mod_ids: Vec<String>,
    },
    /// Remove explicit mods and their orphaned dependencies
    #[command(alias = "rm")]
    Remove {
        /// Mod IDs to remove
        #[arg(value_name = "MODID")]
        mod_ids: Vec<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Print the current persisted configuration or a single field
    #[command(alias = "get")]
    Show {
        /// Configuration field to print
        field: Option<ConfigField>,
    },
    /// Update a persisted configuration value
    Set {
        /// Configuration field to update
        field: ConfigField,
        /// New value for the field
        value: String,
    },
    /// Remove a persisted configuration value
    Unset {
        /// Configuration field to clear
        field: ConfigField,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ConfigField {
    #[value(name = "mods_dir", alias = "mods-dir")]
    ModsDir,
    #[value(name = "update_mirror", alias = "update-mirror")]
    UpdateMirror,
    #[value(name = "gamebanana_mirror", alias = "gamebanana-mirror")]
    GamebananaMirror,
}

impl ConfigField {
    fn name(self) -> &'static str {
        match self {
            Self::ModsDir => "mods_dir",
            Self::UpdateMirror => "update_mirror",
            Self::GamebananaMirror => "gamebanana_mirror",
        }
    }

    fn get(self, config: &Config) -> Option<String> {
        match self {
            Self::ModsDir => config
                .mods_dir
                .as_ref()
                .map(|path| path.display().to_string()),
            Self::UpdateMirror => config.update_mirror.clone(),
            Self::GamebananaMirror => config.gamebanana_mirror.clone(),
        }
    }

    fn set(self, config: &mut Config, value: String) {
        match self {
            Self::ModsDir => config.mods_dir = Some(PathBuf::from(value)),
            Self::UpdateMirror => config.update_mirror = Some(value),
            Self::GamebananaMirror => config.gamebanana_mirror = Some(value),
        }
    }

    fn unset(self, config: &mut Config) {
        match self {
            Self::ModsDir => config.mods_dir = None,
            Self::UpdateMirror => config.update_mirror = None,
            Self::GamebananaMirror => config.gamebanana_mirror = None,
        }
    }
}

fn print_config_field(config: &Config, field: ConfigField) {
    match field.get(config) {
        Some(value) => println!("{} = {:?}", field.name(), value),
        None => println!("{} = None", field.name()),
    }
}

fn print_config(config: &Config) {
    for field in [
        ConfigField::ModsDir,
        ConfigField::UpdateMirror,
        ConfigField::GamebananaMirror,
    ] {
        print_config_field(config, field);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load();

    let active_dir = cli.working_dir.or(config.mods_dir.clone());

    match cli.command {
        Commands::SetModsDir { directory } => {
            config.mods_dir = Some(directory.clone());
            config
                .save()
                .context("Failed to save mods directory to config")?;
            println!("Mods directory set to: {:?}", directory);
        }
        Commands::Config { command } => match command {
            ConfigCommand::Show { field } => {
                if let Some(field) = field {
                    print_config_field(&config, field);
                } else {
                    print_config(&config);
                }
            }
            ConfigCommand::Set { field, value } => {
                field.set(&mut config, value);

                config.save().context("Failed to save configuration")?;
                print_config_field(&config, field);
            }
            ConfigCommand::Unset { field } => {
                field.unset(&mut config);

                config.save().context("Failed to save configuration")?;
                print_config_field(&config, field);
            }
        },
        Commands::Update { mirror } => {
            let dir = active_dir.ok_or_else(|| {
                anyhow::anyhow!("Mods directory not set. Use 'set-mods-dir' or -C to specify one.")
            })?;

            let mirror = mirror
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
        Commands::Tree { loaded, mod_id } => {
            let dir = active_dir.ok_or_else(|| {
                anyhow::anyhow!("Mods directory not set. Use 'set-mods-dir' or -C to specify one.")
            })?;

            tree::run(dir, mod_id, loaded)?;
        }
        Commands::Pull { mod_ids, mirror } => {
            let dir = active_dir.ok_or_else(|| {
                anyhow::anyhow!("Mods directory not set. Use 'set-mods-dir' or -C to specify one.")
            })?;

            let mirror = mirror
                .or_else(|| std::env::var("EVEMODDL_GAMEBANANA_MIRROR").ok())
                .or_else(|| config.gamebanana_mirror.clone())
                .unwrap_or_else(|| "https://gamebanana.com/mmdl".to_string());

            pull::run(dir, mod_ids, mirror).await?;
        }
        Commands::Load { mod_ids } => {
            let dir = active_dir.ok_or_else(|| {
                anyhow::anyhow!("Mods directory not set. Use 'set-mods-dir' or -C to specify one.")
            })?;

            load::run(dir, mod_ids)?;
        }
        Commands::Unload { mod_ids } => {
            let dir = active_dir.ok_or_else(|| {
                anyhow::anyhow!("Mods directory not set. Use 'set-mods-dir' or -C to specify one.")
            })?;

            unload::run(dir, mod_ids)?;
        }
        Commands::Remove { mod_ids } => {
            let dir = active_dir.ok_or_else(|| {
                anyhow::anyhow!("Mods directory not set. Use 'set-mods-dir' or -C to specify one.")
            })?;

            remove::run(dir, mod_ids)?;
        }
    }

    Ok(())
}
