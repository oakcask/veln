# Bibliography Cache Schema

## Layout

Create only the needed parts of this structure:

```text
.bibliography-cache/
  registry.json
  references/
    doi/
    arxiv/
    isbn/
    url/
    title/
  searches/
  collections/
```

Use one directory per reference record or cached search query.

## Keys

Choose the first stable key available:

1. DOI
2. arXiv ID
3. ISBN
4. canonical URL hash
5. normalized title, year, and first-author hash
6. normalized search-query hash

Normalize DOI values to lowercase and remove DOI URL prefixes. Remove spaces
and hyphens from ISBN values. Remove common tracking parameters before hashing
URLs.

## Reference Records

Each record requires `metadata.json` and `citation.md`. It may also include
agent-written `notes.md` and a short `snapshot.md` summary. Cache metadata,
links, permitted short excerpts, citation strings, and summaries; never cache a
full copyrighted work.

Use this metadata shape:

```json
{
  "cache_schema": "bibliography-cache/v1",
  "source_type": "doi",
  "title": "",
  "authors": [],
  "year": null,
  "venue": "",
  "doi": "",
  "arxiv_id": "",
  "isbn": "",
  "url": "",
  "publisher": "",
  "source_published_at": null,
  "source_updated_at": null,
  "accessed_at": "YYYY-MM-DD",
  "fetched_at": "YYYY-MM-DDTHH:MM:SSZ",
  "valid_until": "YYYY-MM-DD",
  "freshness_policy": "scholarly-default",
  "citation_styles": {
    "markdown": "",
    "apa": "",
    "bibtex": ""
  },
  "verification": {
    "method": "",
    "sources_checked": [],
    "confidence": "medium"
  }
}
```

Leave unknown optional values empty or null. `citation.md` should contain only
known or requested citation styles and the source URL.

## Freshness

A record is stale when `valid_until` is before today, the request explicitly
requires current information, or a fast-changing source has no update time.

Default windows:

```text
scholarly-default: 180 days
doi-metadata: 365 days
arxiv: 30 days
book: 365 days
web-page: 90 days
stable-official-page: 180 days
news: 7 days
law-policy-standard: 7 days
software-docs: 30 days
current-request: 0 days
```

Set `accessed_at` to the retrieval date, `fetched_at` to the retrieval
timestamp, and `valid_until` from the selected policy unless the user requests a
stricter rule.

## Search Records

Use:

```text
searches/<query-hash>/
  query.json
  results.json
  summary.md
```

`query.json` records the original and normalized query, language, filters,
requested output style, timestamps, and freshness policy. `results.json`
contains ranked record keys or source URLs rather than duplicated metadata.

## Registry

Keep `registry.json` as a small index from normalized identifiers to record
locations:

```json
{
  "cache_schema": "bibliography-cache/v1",
  "updated_at": "YYYY-MM-DDTHH:MM:SSZ",
  "records": {
    "doi:10.0000/example": "references/doi/<key>",
    "url:<hash>": "references/url/<hash>"
  }
}
```
