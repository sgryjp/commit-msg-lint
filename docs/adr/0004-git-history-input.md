# Use the Git CLI and Temporary Files for Git History Linting

[日本語](0004-git-history-input.ja.md)

**Status: decided and implemented.**

History input lints the commit messages generated for display by `git log`, not the stored raw
bytes. Format validation prioritizes what users see in Git and delegates conversion to UTF-8 to
Git. In exchange, `%B` does not output the part of a message after a NUL, so that part cannot be
linted. If Git cannot convert the bytes and returns invalid UTF-8, report an error rather than
replacing them.

Interpretation of revisions and ranges is delegated to the Git CLI rather than reimplemented with
`git2` or `gix`, to prioritize the same selection semantics as the installed Git. `git cat-file`,
which obtains raw bytes, was also considered, but bidirectional communication and process
management on abnormal termination would be complex, and that precision was judged unnecessary for
format validation of display messages.

Output from `git log` is collected into a temporary file before linting to avoid the burden of
holding an entire large history in memory. Since a process must be started anyway, the additional
I/O from the temporary file is accepted. This is not a decision to impose a fixed upper bound on
overall memory use, including Git and processing of a single message.

History linting is rejected in shallow clones, even for a single commit, because commits may be
missing even if both endpoints of a range resolve. Users must fetch the necessary history beforehand.
Compared with warning and linting only local history, this sacrifices convenience to reduce the risk
of treating an incomplete range as successful. A non-shallow clone alone does not guarantee
completeness of the target range.
