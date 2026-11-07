# Commit Message Linter

This context covers how a team’s shared commit-message conventions are validated.

## Language

**Conventional Commit**:
A commit message that follows the Conventional Commits specification and consists of a type, an
optional scope, and a subject.
_Avoid_: Conventional Commits format, convention-based commit

**Type**:
An identifier at the start of the Header that indicates the kind of change.
_Avoid_: category, kind

**Scope**:
An optional Header component that specifies the target of the change represented by the Type.
_Avoid_: module, area

**Subject**:
A required summary in the Header that briefly describes the change and follows the type and optional
scope.
_Avoid_: description, summary

**Header**:
The first line of a commit message, containing the Conventional Commit type, optional scope, and
subject.
_Avoid_: title, subject line

**Body**:
An optional free-form section that explains or supplements the Header.
_Avoid_: description, message

**Footer**:
An optional section of structured supplementary information, made up of one or more Trailers and
placed after the Header or Body.
_Avoid_: metadata

**Trailer**:
A structured item in the Footer consisting of a token and a value.
_Avoid_: footer entry

**Display Column**:
A unit of visual width based on Unicode display width, used to measure commit-message line length.
_Avoid_: byte length, character count

**Lint Rule**:
A rule that determines whether a commit message complies with the conventions and produces a
diagnostic with a severity when it does not.
_Avoid_: check, validator

**Commit Message Linter**:
A tool that determines whether a commit message complies with the team’s conventions.
_Avoid_: formatter, commit generator
