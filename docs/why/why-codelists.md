# Why code lists?

A clinical **code list** (or *codelist*) is the set of SNOMED CT - or CTV3, ICD-10, dm+d - codes that defines a clinical concept *for a specific purpose*: the codes that mean "has type 2 diabetes" for a particular study, or "currently prescribed an anticoagulant" for a particular alert. They sit underneath almost everything: cohort definitions, audit denominators, risk calculators, decision support, dashboards.

They are also, very often, the **least-engineered artefact in the whole pipeline** - and that is a clinical safety problem.

---

## Code lists are where the errors hide

Documented errors in widely-deployed clinical tools have turned on *which codes were selected*, not on the calculation that consumed them. The clearest UK example is the QRISK2 cardiovascular-risk calculator in TPP's SystmOne: in 2016 the MHRA had the tool temporarily disabled after *code-mapping errors* were identified [[1]](#references) - clinical codes had been omitted and Read 2 codes mis-mapped to CTV3, leaving up to ~270,000 patients with potentially incorrect cardiovascular-risk scores, and with them potentially wrong statin decisions [[2]](#references). The same class of code-selection error has surfaced with other widely-used scores and across more than one GP IT system (stroke-risk scoring such as CHA₂DS₂-VASc among them). In each case the algorithm arithmetic was correct, but the mapping from the real world ("*this* patient has *this* condition") to a set of codes was not. A wrong code list does not throw an error. It silently changes who counts - and you find out, if at all, much later.

This is the uncomfortable truth about code lists: a subtle inclusion or omission is invisible at runtime and can change a denominator, a cohort, or an alert population. The safeguards we take for granted in software - version control, code review, diffs, reproducible builds, a clear record of *why* a change was made - are exactly the safeguards that code lists usually lack.

---

## The status quo, and why it doesn't fit

Most code lists today live as spreadsheets, ad-hoc CSVs, copy-pasted ID columns, or downloads from a web application keyed by an id and a version string. Tools like [OpenCodelists](https://www.opencodelists.org/) are a real step forward and a genuinely valuable community resource - but the artefact still tends to live *outside* the project that uses it. Reuse means an HTTP fetch at run time; the list is not in the same version control as the analysis; review and diff are weak or absent; and curating one often needs specialist or in-house tooling.

The result is that code lists are hard to review, hard to reproduce, hard to audit after the fact, and hard to share as a common currency between teams. Research and direct care frequently re-derive the same lists, in incompatible formats, with no *lingua franca* between them.

---

## What a code list should be

`sct`'s `.codelist` format is an opinionated answer: make the code list a **plain text file you manage like source code**. Seven principles drive the design.

- **Self-contained** - everything about the list, including its metadata and intent, lives in one file (YAML front-matter followed by the concept list). No companion database, no external manifest.
- **Discoverable** - a distinctive `.codelist` extension means lists can be found across an organisation or across GitHub the same way you find any source file (e.g. a path search for `\.codelist$`).
- **Version controlled** - a single flat file that lives in the *same* repository as the code that uses it, so its history, authorship, and review live alongside everything else.
- **Independent** - it is just SCTIDs plus metadata. It does not depend on a particular database, server, application, or even a single terminology; you resolve it against whatever pinned snapshot you choose.
- **Composable** - a list can be built from queries and from other lists, not just hand-typed (see below).
- **Understandable** - simple enough that a clinician can read, review, and reason about it without proprietary tooling or twenty years of terminology experience. Reviewability *by the right person* is the point.
- **Purpose-specific** - every list has a clearly stated intended use. Outside that use it may not behave as intended, and the file says so. A code list is not a universal definition of a concept; it is a definition *for a job*.

---

## How `.codelist` delivers it

A `.codelist` is a UTF-8 file with two parts: **YAML front-matter** (id, title, description, terminology, status, intended use, provenance) and a **body** of concept lines - an SCTID, its term, and an optional inline comment, with explicit `excluded` records for concepts deliberately left out. That shape gives you the principles directly:

- It reads and **diffs like source code**. A pull request that adds three concepts and excludes one is reviewable by a clinician in the same workflow as any other change. *Why* a code is in or out can be recorded next to it.
- It is **independent of infrastructure** - the file is meaningful on its own. `sct` resolves and validates it against a SNOMED snapshot you pin (see [Reproducibility as a clinical safety property](why-build-this.md)), but the list is not welded to that database.
- It is **composable by construction**. Because every way of *finding* concepts in `sct` emits plain SCTIDs, building a list is just plumbing:

    ```bash
    # From an ECL query - the intensional definition
    sct codelist add diabetes.codelist --ecl "<<73211009"

    # From a search, a refset, or anything that speaks SCTIDs
    sct lexical "asthma" --ids        | sct codelist add asthma.codelist -
    sct refset members 447562003 --ids | sct codelist add copy.codelist -
    ```

    Composing a list **out of other lists** - building "cardiovascular disease" from "myocardial infarction" + "stroke" + "peripheral arterial disease" - is the next step on this same road (work in progress; see the [roadmap](https://github.com/pacharanero/sct/blob/main/specs/roadmap.md)). The same principle that makes ECL and refsets powerful, but expressed *legibly, in plain text*.

- It carries **intent, not just members**. The `--ecl` form knows the *expression* that generated a set, which a bare list of ids cannot - the foundation for storing a code list as a re-expandable rule (an *intensional* definition) rather than only a frozen snapshot of members.

---

## A lingua franca

The deeper aim is a **standard, boring, text format** that both research and direct care can share. A flat file with an obvious schema can be transpiled into a Python package, an npm package, a FHIR ValueSet, or an OpenCodelists import - published, peer-reviewed, pinned as a dependency, and reused. Anything agreed and explicit is better than the current status quo of incompatible spreadsheets.

This is the same "data over services" instinct that drives the rest of `sct`: a code list is *data*. It should be copyable, versionable, reviewable, and diff-able like any other data - not locked behind an application or re-derived from scratch by every team that needs it.

---

## What this is not

- **Not a replacement for OpenCodelists.** Its curation, provenance, and community matter; `.codelist` aims to interoperate (import/export), not compete. Direct care and research benefit from speaking the same codes.
- **Not a claim that one list fits all uses.** The *purpose-specific* principle is load-bearing - a list is correct for a stated job, and the file records that job.
- **Not a way around SNOMED licensing.** A `.codelist` of SCTIDs is fine to share; resolving it to terms still needs a licensed SNOMED snapshot.

---

See the [`sct codelist`](../commands/codelist.md) command reference and the [Refsets and Code Lists walkthrough](../walkthrough/refsets-codelists.md) to start building one.

---

## References

1. MHRA. *MHRA information on TPP and QRISK®2*. GOV.UK, 2016. <https://www.gov.uk/government/news/mhra-information-on-tpp-and-qrisk2>
2. Digital Health. *QRisk2 in TPP "fixed" but up to 270,000 patients affected*. 2016. <https://www.digitalhealth.net/2016/06/qrisk2-in-tpp-fixed-but-up-to-270000-patients-affected/>
