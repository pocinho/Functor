# Documentation Structure

## Canonical sources

- `books/dev_book/` is the engineering source of truth.
- `books/user_book/` is the user-perspective guide.
- `docs/README.md` and selected `docs/` compatibility pages only point to the
  books. They are not separate canonical documents.

Keep definitive material in the book chapter that owns it. Put meaningful
architecture choices in `decision/`, delivery checkpoints in `milestone/`,
actionable work in `todo/`, domain/system descriptions in `architecture/`,
and supporting context in `note/`.

## Updating

Update the owning book chapter whenever implementation or an architectural
decision changes. Update the relevant summary when adding a chapter. Do not
silently edit archived material to represent current behavior; if an archive
correction is necessary, preserve the distinction and update the canonical
book.

Build both books before merging documentation changes:

```text
mdbook build books/dev_book
mdbook build books/user_book
```
