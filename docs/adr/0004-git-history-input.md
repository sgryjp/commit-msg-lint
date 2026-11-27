# Use the Git CLI and temporary files to lint Git history

**Status: Decided and implemented.**

For history input, lint the commit messages as rendered by `git log`, rather than the raw bytes
stored in the repository. For format validation, prioritize what users see in Git and let Git
convert the messages to UTF-8. As a result, content after a NUL in a message cannot be checked,
because `%B` does not output content after a NUL. If Git returns bytes that are invalid UTF-8
because it could not convert them, report an error rather than replacing them.

Delegate revision and range interpretation to the Git CLI instead of reimplementing it with
`git2` or `gix`. This prioritizes matching the selection semantics of the installed Git version.
Using `git cat-file` to obtain raw bytes was also considered, but it would complicate
bidirectional communication and process management on abnormal termination. That level of
precision was judged unnecessary for validating the format of rendered messages.

Collect `git log` output in a temporary file before linting it to avoid keeping a large history
entirely in memory. Since this operation already starts a process, the additional I/O from a
temporary file is acceptable. This does not establish a fixed limit on total memory usage,
including Git's own processing or the processing of an individual message.

In a shallow clone, reject history linting, including for a single commit. This is because the
risk of omitting commits is considered more important, even when both ends of a range can be
resolved. Users must fetch the required history beforehand. Compared with warning and linting
only the local history, this sacrifices convenience to reduce the risk of treating an incomplete
range as successful. A repository not being shallow does not, by itself, guarantee that the
target range is complete.
