# Source Decision Records

This directory stores individual implemented source-decision records behind the
category routes in `../`. It is a storage layer, not a first-pass reading
route.

## Read First

- Category route: [../README.md](../README.md).
- Topic map: [../topic-map.md](../topic-map.md).
- Short audit route: [../result-index.md](../result-index.md).

## Storage Shape

- Category pages in `../` provide task routes.
- `result-*.md` files hold one durable decision record each.
- [result-index-full.md](result-index-full.md) is the exhaustive,
  category-grouped list for storage audits.

## Read When

- A category page names one record that explains a boundary in the implemented
  reference.
- You are checking whether a record belongs in a different category route.
- You need the exhaustive category-grouped list in
  [result-index-full.md](result-index-full.md).

## Boundary

Category pages in `../` own task routing. Files here may explain one selected
decision, but they do not replace the implemented reference under
`../../../specification/`.

## Skip Unless Needed

- Do not start here for normal implementation work. Open a category page first,
  then one record only when that page names the relevant topic.
- Do not scan [result-index-full.md](result-index-full.md) when a category page
  already identifies the record.
