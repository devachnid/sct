# sct paths

Print the directories and files `sct` uses by default, and show exactly which path-resolution rule won on this machine.

**When to use:** diagnosing "where does `sct` look for the database / embeddings / config?", or confirming which artefact a bare command (no `--db`) will pick up. The full resolution rules are in [Path resolution](../path-resolution.md); this command shows the *result* of applying them here and now.

---

## Usage

```
sct paths
```

Takes no arguments.

---

## Example

```bash
sct paths
```

```
data home:    ~/.local/share/sct          XDG_DATA_HOME
config home:  ~/.config/sct                XDG_CONFIG_HOME
config file:  ~/.config/sct/config.toml    (none)

database:     ./snomed.db                  current directory
embeddings:   ~/.local/share/sct/data/snomed-embeddings.arrow   data home, canonical name
```

Each row shows the resolved path and, on the right, which rule in the discovery chain produced it (explicit env var → current directory → config file → data-home canonical name → newest matching file). A `─` with "not found" means no artefact of that kind was discovered — pass an explicit `--db` / `--embeddings`, or build one.
