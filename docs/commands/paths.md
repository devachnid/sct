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
data home:       ~/.local/share/sct                                default
config home:     ~/.config/sct                                     default
config file:     ~/.config/sct/config.toml                         exists

database:        ./snomed.db                                       cwd
embeddings:      ~/.local/share/sct/data/snomed-embeddings.arrow    data home, canonical name

trud releases:   ~/.local/share/sct/releases                       3 files
trud data:       ~/.local/share/sct/data                           6 files
```

Each row shows the resolved path and, on the right, which rule in the discovery chain produced it (explicit flag → env var → current directory → config file → data-home canonical name → newest matching file). A `─` with "not found" means no artefact of that kind was discovered - pass an explicit `--db` / `--embeddings`, or build one. The `trud releases` / `trud data` rows show the directories `sct trud download` writes into and how many files are currently there.
