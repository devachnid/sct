# `sct diagram` - concept-definition and hierarchy diagrams

**Status:** ✅ Shipped (slice 1: `tree` / `dot` / `mermaid` for all four views, on any current DB). Deferred: `definition_status` primitive/defined styling (§3.1), DOT attribute-group clusters, built-in `svg` via `layout-rs` (§7 slices 2-3).
**Scope:** Render a SNOMED CT concept's logical definition, or its ancestry / descendants, as a `tree`-style terminal view, Graphviz **DOT**, **Mermaid**, or **SVG** - all from the local `sct` SQLite database, with no server and (for `tree`/`dot`/`mermaid`) no external tools.
**Audience:** A coding agent (and Marcus) implementing this in the `sct` repo.
**Provenance:** Idea harvested from the prior-art Python toolchain `cheethame2017/sct` (Apache-2.0), which produces per-concept Graphviz "focus / ancestor / descendant" diagrams. This is a clean-room Rust reimplementation in `sct`'s local-first, file-based idiom - ideas only, no code.

---

## 1. Why

A picture of a concept's *definition* - its defining IS-A parents and its grouped attribute relationships - is the single fastest way to understand what a SNOMED concept actually means. Today `sct` can `lookup` a concept and list its relationships as text, but nothing draws them. A diagram is:

- **The most demoable gap.** "Here is *Bacterial pneumonia*, and here is its logical definition" as an SVG dropped into a slide or a PR is far more compelling than a table of triples.
- **Cheap to build.** `sct` already holds every row needed (`concept_isa`, `concept_relationships`, `concepts`); the work is emitters, not new data (with one prerequisite - see §3).
- **Complementary, not redundant.** The roadmap's Observable/D3 viewer is an *interactive browser*; this is *static, file-based, pipeline-friendly* output you can commit, diff, and paste into docs. Different job.

---

## 2. CLI surface

```
sct diagram <concept> [--view VIEW] [--format FMT] [--depth N]
                      [--labels LABELS] [--ascii] [-o FILE] [--db PATH]
```

- `<concept>` - focus SCTID (positional). `-` reads a single id from stdin.
- `--view <definition|ancestors|descendants|neighbourhood>` (default `definition`)
  - **definition** - the focus, its direct IS-A parents, and its defining attribute relationships grouped by `group_num`. "What this concept means."
  - **ancestors** - transitive supertypes up the IS-A graph to a root (bounded by `--depth`).
  - **descendants** - subtypes down the IS-A graph (bounded by `--depth`).
  - **neighbourhood** - one hop each way: parents, children, and defining attributes. "What sits around this concept."
- `--format <tree|dot|mermaid|svg>` (default `tree`)
- `--depth N` - max hops for `ancestors`/`descendants`/`neighbourhood` (default: `ancestors` = to root, `descendants` = 1, `neighbourhood` = 1). Ignored for `definition`.
- `--labels <fsn|pt|both|id>` - node captions (default `pt` for compactness; `both` shows `PT (id)`).
- `--ascii` - restrict `tree` output to 7-bit ASCII (`|`, `` `-- ``) instead of Unicode box-drawing, for terminals/pipelines that mangle UTF-8.
- `-o, --output FILE` - write to a file (extension is *not* used to infer format; `--format` is authoritative). Default: stdout.
- `--db PATH` - database; omitted → standard discovery order (`docs/path-resolution.md`).

Composability (per `specs/spec.md`): `tree`/`dot`/`mermaid`/`svg` all write plain text to stdout, so `sct diagram 73211009 --format dot | dot -Tpng -o dm.png` just works. The human-readable summary (node/edge counts, truncation notice) goes to **stderr**, keeping stdout clean.

---

## 3. Data substrate - what exists, and the one prerequisite

| Need | Backing data | Status |
|---|---|---|
| IS-A edges (ancestors/descendants/parents) | `concept_isa(child_id, parent_id)`; optional fast path via `concept_ancestors` (TCT) | **exists** |
| Defining attribute edges | `concept_relationships(source_id, type_id, destination_id, group_num)` | **exists** (needs a DB built by the ECL-era pipeline; see `specs/ecl.md §4`) |
| Node captions | `concepts(id, fsn, preferred_term, active)` | **exists** |
| **Primitive vs fully-defined** styling | `definitionStatusId` (900000000000074008 primitive / …073002 defined) | **NOT persisted - prerequisite** |

**Prerequisite for high-fidelity `definition` view.** The `concepts` table stores no `definition_status`, and `concept_relationships` does not record `characteristicTypeId` (stated vs inferred). Consequences and the additive fix:

1. **Definition status.** To draw primitive concepts differently from fully-defined ones (the standard convention: primitives with a distinct border/shape), add a `definition_status TEXT` column to `concepts`, populated from RF2 (already parsed; currently discarded, exactly like the pre-ECL relationship triples were). Schema-version bump, `#[serde(default)]` on the NDJSON field so older artefacts still load. Until then, `sct diagram` renders all nodes uniformly and prints a stderr note that primitive/defined styling needs a rebuilt DB.
2. **Stated vs inferred.** `concept_relationships` holds the RF2 Snapshot **inferred** relationships (the long normal form). That is the correct and sufficient substrate for a "what this concept means" diagram; the stated form / OWL axioms are a separate, later concern (see the SCG/OWL roadmap item). Document that the `definition` view shows the *inferred* definition.

Neither prerequisite blocks the `ancestors`/`descendants`/`neighbourhood` views or the `tree` format - those work on any current database.

---

## 4. Output formats

All formats render the same in-memory graph (nodes = concepts, edges = typed `is a` / attribute relationships). One builder walks the substrate into a `Graph { nodes, edges }`; four back-ends serialise it.

### 4.1 `tree` (default, zero-dependency, terminal-first)

Unix `tree`-style, Unicode box-drawing by default (`├──`, `└──`, `│`), `--ascii` fallback. Edges are labelled by their relationship type so an attribute tree is readable.

`--view definition` of *Bacterial pneumonia*:

```
Bacterial pneumonia (53084003)  [primitive]
├─ is a ── Infectious pneumonia (301810000)
├─ is a ── Bacterial lower respiratory infection (50417007)
└─ role group 1
   ├─ Causative agent ── Bacterium (409822003)
   ├─ Finding site ──── Lung structure (39607008)
   └─ Pathological process ── Infectious process (441862004)
```

`--view descendants --depth 2` of *Diabetes mellitus*:

```
Diabetes mellitus (73211009)
├── Type 1 diabetes mellitus (46635009)
│   ├── Brittle diabetes mellitus (438880008)
│   └── ...
├── Type 2 diabetes mellitus (44054006)
│   └── ...
└── ...
```

IS-A is a DAG, so a concept can appear under more than one parent. Like `tree`'s handling of hard links, a node already expanded elsewhere is printed once with its subtree, and subsequent occurrences are marked `↑ (see above)` rather than re-expanded - this both prevents unbounded blow-up and signals the multiple-parent structure honestly. Truncation by `--depth` prints `… (N more, use --depth)`.

### 4.2 `dot` - Graphviz (high-fidelity path)

Emit standard DOT text. Attribute groups become `subgraph cluster_N` boxes; primitive concepts get a distinct node style (once §3.1 lands); IS-A and attribute edges are visually distinguished (solid vs by colour/label). This is the **recommended path for publication-quality images**, because real Graphviz `dot` has the best layout engine and rasterises directly (§5). No dependency inside `sct` - we only emit text.

### 4.3 `mermaid` - docs-native

Emit a Mermaid `graph TD`. Renders natively in GitHub Markdown and the MkDocs site (the roadmap already wants Mermaid architecture diagrams), so a concept diagram can be embedded in docs with no image build step at all.

### 4.4 `svg` - self-contained, no external tools

Render SVG **in-process via the pure-Rust `layout-rs` crate** (`0.1.2` at time of writing; parses DOT / builds a graph and renders SVG with a Sugiyama layout, no Graphviz binary required). This keeps `sct`'s "no special tools required" promise for users who just want an image and don't have Graphviz installed.

Honest caveat to document: `layout-rs` does **not** render nested-graph clusters or HTML labels, so the built-in SVG shows attribute groups via edge labelling/colour rather than cluster boxes, and its layout quality on large graphs is below Graphviz's. For best results on anything non-trivial, use `--format dot | dot -Tsvg`. Implement `svg` behind a `--features diagram-svg` cargo feature if the dependency weight is unwelcome in the default build; `dot`/`mermaid`/`tree` stay dependency-free and always on.

---

## 5. Rendering SVG/DOT to PNG or JPG (for tutorials & slides)

Document these recipes in `docs/commands/diagram.md`. Two routes: let Graphviz rasterise directly (simplest if you have it), or convert the SVG.

**Direct raster via Graphviz** (no SVG middle step) - recommended for slides:

```bash
# PNG at presentation DPI, white background
sct diagram 53084003 --format dot | dot -Tpng -Gdpi=200 -Gbgcolor=white -o pneumonia.png
# JPG
sct diagram 53084003 --format dot | dot -Tjpg -Gdpi=200 -o pneumonia.jpg
```

**Convert an SVG** produced by `sct diagram … --format svg -o concept.svg` - pick whichever tool is already installed:

| Tool | Command | Notes |
|---|---|---|
| librsvg | `rsvg-convert -o out.png --zoom 2 concept.svg` | fast, faithful; `--zoom`/`--dpi-x` for resolution |
| resvg | `resvg --zoom 2 concept.svg out.png` | pure-Rust, matches our renderer well |
| ImageMagick | `magick -density 200 -background white concept.svg out.png` | `-background white` flattens transparency for slides; JPG via `out.jpg` |
| Inkscape | `inkscape concept.svg --export-type=png --export-dpi=200 -o out.png` | heaviest, best CSS support |
| cairosvg | `cairosvg concept.svg -o out.png --output-width 1600` | Python, no system libs |

Guidance to include: use **PNG** for diagrams (sharp edges/text; lossless), reserve JPG for photos; render at **~200 dpi / 2× zoom** for crisp projection; add a **white background** (`-Gbgcolor=white` / `-background white`) since default SVG transparency shows through dark slide themes.

---

## 6. Testing

- **Graph builder unit tests** - over a small synthetic fixture DB (a hand-made hierarchy + a couple of grouped attribute relationships): assert the node/edge set for each `--view`, and that multi-parent nodes are de-duplicated with the `↑` marker rather than duplicated or looped.
- **Format snapshot tests** (`insta`) - freeze the `tree`, `dot`, and `mermaid` output for one canonical concept so accidental format drift is caught. `--ascii` gets its own snapshot.
- **SVG smoke test** (behind `diagram-svg`) - render the fixture to SVG and assert it is well-formed XML containing the focus id; do not pixel-compare.
- **Depth/truncation** - `--depth` bounds are respected and the truncation notice appears.
- CI gates unchanged: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, full suite.

---

## 7. Sequencing

1. **Slice 1** - graph builder + `tree` and `mermaid` formats for all four views; `--depth`, `--labels`, `--ascii`. Zero new dependencies, works on any current DB. Ships the demo value immediately.
2. **Slice 2** - `dot` format with attribute-group clusters and IS-A/attribute edge styling; the `definition_status` schema addition (§3.1) so primitive/defined styling is correct; `docs/commands/diagram.md` with the §5 conversion recipes.
3. **Slice 3** - built-in `svg` via `layout-rs` behind `--features diagram-svg`.
4. **Later** - MCP `snomed_diagram` tool returning DOT/Mermaid for a concept; wiring into `sct serve --ui`; through-time / MRCM diagram variants (separate roadmap items).

---

## 8. References

- `specs/ecl.md` - the `concept_relationships` substrate (§4) this reuses.
- `specs/roadmap.md` - "Ideas harvested from prior-art Python tooling" (source of this item) and the Observable/D3 viewer it complements.
- `layout-rs` - <https://github.com/nadavrot/layout> (pure-Rust DOT→SVG).
- Graphviz DOT language - <https://graphviz.org/doc/info/lang.html>.
- Mermaid flowchart syntax - <https://mermaid.js.org/syntax/flowchart.html>.
- Prior art: `cheethame2017/sct` - <https://bitbucket.org/cheethame2017/sct/src/development/>.
