## Commands
- Format with `cargo +nightly fmt`; `.rustfmt.toml` uses unstable rustfmt options.
- Run `cargo clippy -- -D warnings` before committing so warnings fail locally.
- Run tests with `cargo test`; focus one test with `cargo test <test_name>`.
- Use `cargo run -- -C ./test_mods <command>` for filesystem/manual CLI checks; `./test_mods/` is gitignored.

## Project Shape
- Single-crate Rust 2024 CLI; `src/main.rs` wires Clap commands to one module per command.
- No README, CI, task runner, or workspace config currently exists; trust `Cargo.toml`, `.rustfmt.toml`, and `src/`.
- Persisted config is TOML from `directories::ProjectDirs`, e.g. `~/.config/evemoddl/config.toml` on Linux.
- The active mods directory is `-C <dir>` if supplied, otherwise persisted `mods_dir`; there is no env var for `mods_dir`.

## CLI State
- `update` must run before commands needing mirror metadata; it writes `.evemoddl/mod_dependency_graph.yaml` and `.evemoddl/everest_update.yaml` under the active mods dir.
- `pull` downloads archives to `.evemoddl/files/<ModID>.zip.tmp`, verifies update-list `xxHash`, then renames to `.evemoddl/files/<ModID>.zip` and updates `.evemoddl/files.toml`.
- `pull` records `version`, `is_explicit`, and `loaded`; pulling an already-current dependency should still promote it to explicit.
- `load` creates hard links from `.evemoddl/files/*.zip` into the active mods dir, skips existing target zips, and marks mods loaded.
- `unload` and `remove` operate only on explicit mods directly; dependency links/state are cleaned based on remaining explicit mods.
- Exact ModID args use `src/mod_id.rs`: letters match case-insensitively, and spaces/special chars may be typed as themselves, `-`, or `_`; ambiguous matches are errors.

## Dependency Rules
- Resolve only `Dependencies`; `OptionalDependencies` are informational and never auto-installed.
- Always skip `Celeste`, `Everest`, and `EverestCore`; they are core system deps, not managed archives.
- `remove` refuses direct removal of dependency-only mods and auto-removes orphaned dependencies.

## Mirrors
- Update mirror default: `https://everestapi.github.io/updatermirror/`.
- GameBanana mirror default: `https://gamebanana.com/mmdl`.
- Mirror priority is CLI `-m` over env var over config over default.
- Env vars: `EVEMODDL_UPDATE_MIRROR`, `EVEMODDL_GAMEBANANA_MIRROR`.

## Command Map
- `set-mods-dir <path>` stores the default mods dir.
- `config show|get [field]`, `config set <field> <value>`, `config unset <field>` manage `mods_dir`, `update_mirror`, and `gamebanana_mirror`.
- `update [-m <mirror>]`, `search <query>`, `tree [--loaded|MODID]`, `pull|download|dl [-m <mirror>] <MODID>...`, `load <MODID>...`, `unload <MODID>...`, `remove|rm <MODID>...`.
