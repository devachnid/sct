# SNOMED CT's "free" releases: GPS and IPS

SNOMED International publishes two artefacts that are often described as
freely available: the **Global Patient Set (GPS)** and the **IPS
Terminology** (International Patient Summary refset). At first glance
these look like they might solve the bootstrapping problem - ship a
usable SNOMED CT database inside `sct` so people can try it without
navigating TRUD or MLDS first.

We investigated both carefully. One of them turns out to be more useful
than it first appears.

## The Global Patient Set (GPS)

The GPS is a flat TSV file containing every active SNOMED CT concept
identifier, its FSN, and its US English preferred term. It also includes
concepts inactivated since January 2012. It is released under
**CC BY-ND 4.0** (Creative Commons Attribution-NoDerivatives), which
means:

- You may share the file verbatim, including commercially.
- You must credit SNOMED International.
- **You may not create derivatives.**

### Can we convert the format?

The NoDerivatives clause sounds like it prohibits converting GPS data
from TSV into NDJSON or SQLite - i.e. everything `sct` does. However,
the Creative Commons FAQ is
[surprisingly clear](https://creativecommons.org/faq/#when-is-my-use-considered-an-adaptation)
on this point:

> *"All CC licenses allow the user to exercise the rights permitted
> under the license in **any format or medium**. Those changes are
> **not considered adaptations** even if applicable law would suggest
> otherwise."*

And separately:

> *"Can I take a CC-licensed work and use it in a different format?
> Yes. [...] This is true even in our NoDerivatives licenses."*

So converting GPS from TSV to SQLite, NDJSON, or any other mechanical
format is **not** an adaptation under CC BY-ND 4.0. We could legally
redistribute the GPS data in a different container format, provided we
don't add creative expression or modify the content itself. Building
an FTS index over the terms, for instance, is a grey area - it's
arguably a mechanical transformation, but it could also be seen as
creating a new database arrangement that goes beyond format shifting.

In practice, this means a `sct gps` command that downloads and converts
the GPS into a local SQLite database is likely fine. Bundling a
pre-built SQLite database inside the `sct` binary for redistribution is
more cautious territory, but probably also defensible as a format shift.

### What the GPS doesn't include

The GPS is deliberately stripped of everything that makes SNOMED CT a
terminology rather than a code list:

- No IS-A hierarchy
- No attribute relationships or logical definitions
- No synonyms (only one preferred term per concept)
- No reference sets
- No historical associations
- No language translations beyond US English

On its own, it is a flat lookup table. You can go from a SCTID to a
display string and back. The `sct` features that depend on semantic
structure - hierarchy walking, subsumption, reference-set browsing -
will not work against GPS data alone.

### But the GPS is not a dead loss

The namespace is the hard part. The GPS gives you every active SNOMED CT
concept identifier, freely and legally. That on its own unlocks
several things:

- **Free-text search over SNOMED codes in US English.** Any system that
  needs to let a user type a clinical term and get back a SCTID can do
  so with GPS data and an FTS index - no Affiliate License required.

- **A foundation for codelist-driven terminology.** The missing
  hierarchy and subsumption can be partially replaced by curated,
  versioned codelists. The `.codelist` format that `sct` already
  supports is designed for exactly this: human-authored, auditable
  groupings of SNOMED concepts. A GPS database combined with a library
  of open codelists gives you much of the practical utility of
  full SNOMED CT - not the formal ontology, but the clinical groupings
  that real systems actually use at the point of care.

- **A bootstrapping experience for new users.** Shipping (or
  downloading) the GPS means someone can try `sct lookup`, `sct search`,
  and basic codelist workflows without first navigating TRUD or MLDS
  registration. The experience is limited compared to a full edition,
  but it is far better than nothing.

This pattern - GPS as namespace, codelists as structure - has
implications beyond `sct`. Any open-source clinical system could ship
GPS data for code search, and layer community-maintained codelists on
top for the curated groupings that hierarchy and ECL provide in
licensed SNOMED CT.

## The IPS Terminology

The IPS Refset is a curated subset of roughly 5,000–8,000 concepts used
by the HL7 International Patient Summary standard. Despite the word
"free" appearing in some of the surrounding documentation, the IPS
Refset is distributed through
[NLM's UMLS Terminology Services](https://www.nlm.nih.gov/healthit/snomedct/international.html)
and SNOMED International's
[MLDS](https://mlds.ihtsdotools.org/) under the **standard SNOMED CT
Affiliate License**. NLM's
[licensing page](https://www.nlm.nih.gov/healthit/snomedct/snomed_licensing.html)
confirms that all SNOMED CT content distributed via UMLS - including the
IPS Refset - is subject to the Affiliate License terms (incorporated as
Appendix 2 of the UMLS Metathesaurus License). It is not a free set in
any meaningful sense. Redistributing it inside a tool like `sct` is
explicitly prohibited.

## The deeper problem: openwashing (with a silver lining)

These two artefacts are, we think, a case of openwashing. They carry the
aesthetics of open licensing - Creative Commons badges, public download
pages, talk of "global access" - while being structured so that the
full terminology remains firmly behind a paywall.

The GPS in particular is revealing. SNOMED International clearly
understands the demand for openly accessible terminology. The GPS
responds to that demand by publishing every concept identifier and term,
which *sounds* generous. The NoDerivatives clause turns out to be less
restrictive than it first appears - format shifting is explicitly
permitted - but the deliberate exclusion of all semantic content means
that the result is a shadow of the real thing. You get 400,000+
identifiers with display names, but none of the relationships that give
those identifiers meaning.

The IPS Refset goes further in the wrong direction: it is simply not
free at all, despite the surrounding language suggesting otherwise.

We think this reflects an organisation that knows SNOMED CT should be
openly available - clinical terminology is public infrastructure, not a
luxury good - but that has not yet found the institutional will to make
it happen. The GPS and IPS feel like compromise artefacts: small enough
to get past internal objections, restricted enough to protect the
existing licensing revenue.

The irony is that by releasing the namespace - every concept ID and its
preferred term - SNOMED International may have given away more than it
intended. The namespace is the part that is genuinely hard to
replicate. Hierarchy and groupings, by contrast, can be rebuilt by
clinical communities through curated codelists, and those codelists
can be freely licensed without any dependency on SNOMED International's
permission. The GPS was probably designed as a minimal concession; it
may turn out to be the foundation for something much more open.

## What this means for sct users

If you are in a SNOMED CT member country (the UK, US, Australia, and
many others), you already have access to the full edition through your
national release centre - TRUD in the UK, NLM in the US, and so on.
That is the data you should use with `sct`, and the
[getting started guide](walkthrough/getting-started.md) walks you
through obtaining it. A full edition gives you everything: hierarchy,
synonyms, reference sets, maps, and the complete semantic machinery.

If you are new to `sct` or in a non-member country, the GPS provides
a useful starting point. We plan to add a `sct gps` command that
downloads and converts the GPS into a local database, giving you
free-text SNOMED search and basic codelist workflows out of the box.
It is a fraction of what a full edition provides, but combined with
open codelists it covers a surprising amount of real clinical ground.

## References

- [GPS Implementation Guide](https://github.com/SNOMED-Documents/snomed-gps-ig)
  (SNOMED International, CC BY 4.0 - the *guide* is open; the *data*
  is CC BY-ND)
- [CC BY-ND 4.0 Legal Code](https://creativecommons.org/licenses/by-nd/4.0/legalcode)
- [Creative Commons FAQ - "When is my use considered an adaptation?"](https://creativecommons.org/faq/#when-is-my-use-considered-an-adaptation)
  (confirms format shifting is permitted under ND)
- [SNOMED CT Affiliate License](https://www.snomed.org/affiliate-license)
  (governs the IPS Refset and all non-GPS SNOMED CT content)
- [GPS download page](https://gps.snomed.org/)
