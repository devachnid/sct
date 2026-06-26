# Get Your Own Terminology Server In 2 Commands

Run a FHIR R4 SNOMED CT terminology server on a clean VPS with Docker Compose.
The first boot downloads the configured NHS TRUD release, builds the SQLite
database, and starts `sct serve`.

## Prerequisites

- Docker with the Compose plugin.
- An NHS TRUD account subscribed to the SNOMED CT UK Monolith Edition.
- Your TRUD API key.

The TRUD key is only used by your container to download the release. `sct` does
not redistribute SNOMED CT content.

## Fast Path

```bash
git clone https://github.com/pacharanero/sct.git
cd sct && TRUD_API_KEY=your-key-here docker compose up --build
```

The server is available at:

```text
http://localhost:8080/fhir
```

For a real VPS, prefer a `.env` file so the API key is not left in shell history:

```bash
git clone https://github.com/pacharanero/sct.git
cd sct
cp .env.example .env
$EDITOR .env
docker compose up --build
```

Set:

```text
TRUD_API_KEY=your-key-here
```

## What First Boot Does

If the Docker volume does not already contain a database, the entrypoint runs:

```bash
sct trud download --edition uk_monolith --skip-if-current --pipeline --refsets all --locale en-GB
```

That produces a SNOMED SQLite database and then starts:

```bash
sct serve --host 0.0.0.0 --port 8080 --fhir-base /fhir
```

Subsequent starts reuse the existing database in the `sct-data` Docker volume,
so they skip the TRUD download and build step.

## Check It Works

```bash
curl 'http://localhost:8080/fhir/metadata'
```

Look up a concept:

```bash
curl 'http://localhost:8080/fhir/CodeSystem/$lookup?system=http://snomed.info/sct&code=22298006'
```

Expand an ECL ValueSet:

```bash
curl 'http://localhost:8080/fhir/ValueSet/$expand?url=http://snomed.info/sct?fhir_vs=ecl/%3C%3C73211009&count=10'
```

## Configuration

Edit `.env` to change the bootstrap defaults:

| Variable | Default | Description |
|---|---|---|
| `TRUD_API_KEY` | - | Required for first boot unless you provide an existing database. |
| `SCT_TRUD_EDITION` | `uk_monolith` | Built-in TRUD edition to download. |
| `SCT_REFSETS` | `all` | `all` enables ICD-10 / OPCS-4 maps and concept history. |
| `SCT_LOCALE` | `en-GB` | Preferred-term locale. |
| `SCT_INCLUDE_INACTIVE` | `false` | Set `true` to retain inactive concepts. |
| `SCT_PORT` | `8080` | Host port mapped to the container. |

## Operations

Stop the server:

```bash
docker compose down
```

Upgrade after pulling newer code:

```bash
git pull
docker compose up --build
```

Force a fresh download/build by removing the volume:

```bash
docker compose down -v
docker compose up --build
```

The volume contains licensed SNOMED CT data. Do not publish it as part of an
image or commit exported database files to git.

## FHIR Surface

The Docker setup runs the same server as [`sct serve`](commands/serve.md),
including:

- `CodeSystem/$lookup`
- `CodeSystem/$validate-code`
- `CodeSystem/$subsumes`
- `ValueSet/$expand`
- `ValueSet/$validate-code`
- `ConceptMap/$translate` when the database has crossmaps loaded

For the full operation reference, see [`sct serve`](commands/serve.md).
