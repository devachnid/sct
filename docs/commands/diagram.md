# sct diagram

Draw a SNOMED CT concept - its logical definition, its ancestry, or its descendants - as a terminal `tree`, Graphviz **DOT**, **Mermaid**, or built-in **SVG**. All output is plain text on stdout, so it pipes cleanly into files and other tools.

**When to use:** you want to *see* a concept's structure rather than read a table of relationships - to understand a definition, sanity-check a hierarchy, or drop a picture into docs, a slide, or a PR.

---

## Usage

```
sct diagram <CONCEPT> [--view <VIEW>] [--format <FORMAT>]
            [--depth <N>] [--labels <STYLE>] [--ascii] [-o <FILE>] [--db <FILE>]
```

| Argument / Flag | Default | Description |
|---|---|---|
| `<CONCEPT>` | *(required)* | Focus concept SCTID. Pass `-` to read a single id from stdin. |
| `--view <VIEW>` | `definition` | `definition`, `ancestors`, `descendants`, or `neighbourhood` (see below). |
| `--format <FORMAT>` | `tree` | `tree`, `dot`, `mermaid`, or `svg` (`svg` requires the `diagram-svg` build feature). |
| `--depth <N>` | view-dependent | Max hops for `ancestors` / `descendants` (default: ancestors to root, descendants 1). |
| `--labels <STYLE>` | `pt` | Node captions: `pt`, `fsn`, `both`, or `id`. |
| `--ascii` | off | Use 7-bit ASCII tree glyphs instead of Unicode box-drawing. |
| `-o, --output <FILE>` | stdout | Write to a file (the format is set by `--format`, not the extension). |
| `--db <FILE>` | discovered (see [Path resolution](../path-resolution.md)) | SQLite database from `sct sqlite`. |

The node/edge count is written to **stderr**, so it never pollutes a pipe.

### Views

- **definition** - the focus, its IS-A parents, and its defining attribute relationships (grouped into role groups). "What this concept means."
- **ancestors** - transitive supertypes up to a root.
- **descendants** - subtypes downward (bounded by `--depth`).
- **neighbourhood** - one hop each way: parents, children, and defining attributes.

Attribute relationships (the `definition` and `neighbourhood` views) need a database built with schema v4+ (the `concept_relationships` table); without it the IS-A structure still renders and a note is printed. Databases built with schema v6+ also distinguish primitive concepts (dashed slate border) from fully-defined concepts (filled green) in DOT output. Older databases still render, with a note explaining how to rebuild.

---

## Examples

```bash
# A concept's logical definition in the terminal
sct diagram 22298006 --view definition
# Myocardial infarction (22298006)
# ├── is a: Myocardial necrosis (251061000)
# ├── is a: Ischaemic heart disease (414545008)
# └── role group 1
#     ├── Associated morphology: Infarct (55641003)
#     └── Finding site: Myocardium structure (74281007)

# Descendants, three levels deep
sct diagram 73211009 --view descendants --depth 3

# Ancestry as ASCII (safe for logs / plain terminals)
sct diagram 44054006 --view ancestors --ascii

# Mermaid for a docs page (renders natively on GitHub and this site)
sct diagram 73211009 --view neighbourhood --format mermaid

# Graphviz DOT to a file
sct diagram 404684003 --view descendants --format dot -o clinical.dot

# Definition diagram with role-group boxes and primitive/defined node styling
sct diagram 53084003 --view definition --format dot | dot -Tsvg -o pneumonia.svg

# Built-in SVG (no Graphviz executable; requires --features diagram-svg)
sct diagram 53084003 --view definition --format svg -o pneumonia.svg
```

---

## Turning a diagram into a PNG or JPG

For tutorials and slides you'll want a raster image. Two routes.

### Route 1 - let Graphviz rasterise directly (simplest)

If you have Graphviz installed, `dot` reads the DOT output and writes PNG/JPG in one step - no intermediate SVG:

```bash
# PNG at presentation resolution, white background
sct diagram 53084003 --format dot | dot -Tpng -Gdpi=200 -Gbgcolor=white -o concept.png

# JPG
sct diagram 53084003 --format dot | dot -Tjpg -Gdpi=200 -o concept.jpg

# SVG (scales cleanly for the web)
sct diagram 53084003 --format dot | dot -Tsvg -o concept.svg
```

### Route 2 - convert an SVG

If you produced an SVG (`… -Tsvg` above or `--format svg`), convert it with whichever tool you already have:

| Tool | Command | Notes |
|---|---|---|
| librsvg | `rsvg-convert -o out.png --zoom 2 concept.svg` | fast, faithful |
| resvg | `resvg --zoom 2 concept.svg out.png` | pure-Rust, no system libs |
| ImageMagick | `magick -density 200 -background white concept.svg out.png` | `-background white` flattens transparency; JPG via `out.jpg` |
| Inkscape | `inkscape concept.svg --export-type=png --export-dpi=200 -o out.png` | best CSS support |
| cairosvg | `cairosvg concept.svg -o out.png --output-width 1600` | Python |

**Tips for slides:** use **PNG** for diagrams (sharp text and edges; JPG is for photos), render at **~200 dpi / 2× zoom** so it stays crisp on a projector, and add a **white background** (`-Gbgcolor=white` / `-background white`) so the default transparency doesn't vanish against a dark slide theme.

### Enabling built-in SVG

The SVG format is present in binaries built with the optional pure-Rust renderer: `cargo install --path . --features diagram-svg` (or `--features full`). Built-in SVG preserves node and attribute-edge styling, but `layout-rs` does not support Graphviz cluster boxes; use `--format dot | dot -Tsvg` when publication-quality role-group clusters or Graphviz's more mature layout are important.

---

See [`spec/commands/diagram.md`](https://github.com/pacharanero/sct/blob/main/spec/commands/diagram.md) for the design record.
