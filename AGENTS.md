## Lint and Format
- Run `cargo clippy` and fix all errors and warnings before committing
- Format with nightly: `cargo +nightly fmt` (required - .rustfmt.toml uses unstable_features)

## Testing
- Filesystem tests must use `./test_mods/` (gitignored)
- No unit tests currently exist; use standard `cargo test` when added

## Project Structure
- Single crate CLI for managing Celeste (Everest) game mods
- Config stored in system config dir (~/.config/evemoddl/ on Linux)
- Mod metadata cached in `.evemoddl/` within the active mods directory:
  - `files.toml` - installed mod state
  - `everest_update.yaml` - update list from mirror
  - `mod_dependency_graph.yaml` - dependency graph from mirror
  - `files/` - downloaded mod archives

## CLI Commands
- `set-mods-dir <path>` - Set default mods directory (stored in config)
- `update` - Download latest mod lists from mirror (requires --mirror or EVEMODDL_UPDATE_MIRROR)
- `search <query>` - Search installed update list for mods
- `pull <modid>...` - Download mods and dependencies from GameBanana (requires --mirror or EVEMODDL_GAMEBANANA_MIRROR)
- Use `-C <dir>` to override working directory for any command

## Mirrors
- Default update mirror: https://everestapi.github.io/updatermirror/
- Default GameBanana mirror: https://gamebanana.com/mmdl
- Override priority: CLI arg > env var > config file > default
