# Test fixtures

Two groups, each a flat folder of `.md` files:

- `1x/` holds the 1.x `examples/*.md` files, unchanged.
- `edge/` holds the edge cases listed in the 2.0 spec (section 12): CRLF, a
  BOM, non-Latin text, block scalars, anchors and aliases, quotes that are
  needed and ones that are not, empty collections, nested flow collections,
  unclosed or empty front matter, duplicate keys, 129-level nesting, disable
  comments and a delimiter with a trailing space.

The over-1 MiB input is generated inside the M2 tests and is never committed.

`/.gitattributes` marks everything here as `-text`, so Git never converts line
endings. `edge/crlf.md` is CRLF on purpose; every other fixture is LF only. The
test `fixture_line_endings_are_preserved` checks both.

The recorded 1.7.1 results for each fixture live in `tests/parity/expected-1x/`
(see `cargo xtask parity-record`).
