# Measuring Line Length with Display Columns

Measure line length in Display Columns using the `unicode-width` crate’s string-width algorithm,
not by byte count or grapheme-cluster count. Wide characters, such as Japanese characters, count as
two columns. Ambiguous-width characters count as one column by default, but can be configured to
count as two. Grapheme-cluster counts better reflect the number of characters users perceive, but
they count `a` and `あ` as the same length, so they are not suitable for commit-log display-width
limits.

Because terminal and font rendering cannot be matched exactly, pin the crate and Unicode data
versions for each release and document any updates.
