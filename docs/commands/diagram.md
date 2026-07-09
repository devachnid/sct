# sct diagram

Draw a SNOMED CT concept - its logical definition, its ancestry, or its descendants - as a terminal `tree`, Graphviz **DOT**, or **Mermaid**. All output is plain text on stdout, so it pipes into an image with one more command (see [Turning a diagram into a PNG or JPG](#turning-a-diagram-into-a-png-or-jpg)).

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
| `--format <FORMAT>` | `tree` | `tree`, `dot`, or `mermaid`. |
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

Attribute relationships (the `definition` and `neighbourhood` views) need a database built with schema v4+ (the `concept_relationships` table); without it the IS-A structure still renders and a note is printed.

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

If you produced an SVG (`… -Tsvg` above, or a future `--format svg`), convert it with whichever tool you already have:

| Tool | Command | Notes |
|---|---|---|
| librsvg | `rsvg-convert -o out.png --zoom 2 concept.svg` | fast, faithful |
| resvg | `resvg --zoom 2 concept.svg out.png` | pure-Rust, no system libs |
| ImageMagick | `magick -density 200 -background white concept.svg out.png` | `-background white` flattens transparency; JPG via `out.jpg` |
| Inkscape | `inkscape concept.svg --export-type=png --export-dpi=200 -o out.png` | best CSS support |
| cairosvg | `cairosvg concept.svg -o out.png --output-width 1600` | Python |

**Tips for slides:** use **PNG** for diagrams (sharp text and edges; JPG is for photos), render at **~200 dpi / 2× zoom** so it stays crisp on a projector, and add a **white background** (`-Gbgcolor=white` / `-background white`) so the default transparency doesn't vanish against a dark slide theme.

---

See [`spec/commands/diagram.md`](https://github.com/pacharanero/sct/blob/main/spec/commands/diagram.md) for the design record.
