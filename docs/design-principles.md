# Design principles

- Measure display width using East Asian character widths, not character counts. This better
  reflects commit-log display, although terminal and font rendering can differ
  ([ADR](adr/0001-display-column-measurement.md)).
- Separate recommended limits from required ones with warnings and errors, giving agents discretion
  to preserve meaning.
