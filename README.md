# Validate `.git-blame-ignore-revs`

Validate a [file used by Git for ignoring revisions in `git blame` output](https://www.michaelheap.com/git-ignore-rev/).

By convention, this file is named `.git-blame-ignore-revs` and is located at the top level or base of the repository.

## Installation

The package is [currently not available on PyPI](https://github.com/berquist/validate-git-blame-ignore-revs/issues/11).
In the meantime, it is available in [VCS format](https://pip.pypa.io/en/stable/topics/vcs-support/).
For example, to use with [uv](https://docs.astral.sh/uv/guides/tools/#requesting-specific-versions):

```bash
uvx git+https://github.com/berquist/validate-git-blame-ignore-revs@v0.1 --help
```

## Recommended usage

For general use, we recommend installation as a pre-commit hook.
If the file isn't modified, the hook won't run, so it won't slow down pre-commit.
The actual time to run will vary depending on the arguments used and the size of the ignore file,
but on [this commit](https://github.com/cclib/cclib/commit/188c8328d80466fa4d0a4f15c486864072c6ae86)
with the strictest settings it takes under five seconds.

For the first usage in a repository, it would be better to either use it directly from the command line
or run it with pre-commit using the `-a` flag.

## Arguments and usage as a command-line tool

```console
usage: validate-git-blame-ignore-revs [-h] [--call-git] [--strict-comments] [--strict-comments-git] [--pre-commit-ci] file_path

Validate a .git-blame-ignore-revs file.

positional arguments:
  file_path             Path to the .git-blame-ignore-revs file.

optional arguments:
  -h, --help            show this help message and exit
  --call-git            Ensure each commit is in the history of the checked-out branch.
  --strict-comments     Require each commit line to have one or more comment lines above it.
  --strict-comments-git
                        Ensure the comment above each commit matches the first part of the commit message. Requires --strict-comments and --call-git.
  --pre-commit-ci       Ensure all commits authored by pre-commit-ci[bot] are present in the file. Requires --call-git.
```

When called without any optional arguments,
`validate-git-blame-ignore-revs` will only check that the syntax of the ignore file is correct.
This means each line is either whitespace, is a comment (starts with `#`),
or is a 40-character hex string.
It may be useful for fast sanity checks in order to avoid calling Git.
All other arguments except for `--strict-comments` require calling Git,
since the information requires checking history.

## Use as a pre-commit hook

Add the following to your `.pre-commit-config.yaml` under the `repos` list:

```yaml
  - repo: https://github.com/berquist/validate-git-blame-ignore-revs
    rev: v0.1
    hooks:
      - id: validate-git-blame-ignore-revs

```

The default file to look for is `.git-blame-ignore-revs` at the top level of the repository.
The default arguments are the strictest available (`--call-git --strict-comments --strict-comments-git --pre-commit-ci`).
The full configuration can be seen in `.pre-commit-hooks.yaml`.

### Changing options via arguments

In order to change the arguments used, [specify `args`](https://pre-commit.com/#passing-arguments-to-hooks).
For example, to only check the syntax of the file and that each commit is present in Git history,

```yaml
  - repo: https://github.com/berquist/validate-git-blame-ignore-revs
    rev: v0.1
    hooks:
      - id: validate-git-blame-ignore-revs
        args: [--call-git]
```

### Changing filename

If your ignore revs file is named something other than `.git-blame-ignore-revs`,
override the regular expression used for [`files`](https://pre-commit.com/#hooks-files).
For example, if your file has the idiomatic name but is located in a `dev` subdirectory,

```yaml
  - repo: https://github.com/berquist/validate-git-blame-ignore-revs
    rev: v0.1
    hooks:
      - id: validate-git-blame-ignore-revs
        files: '^dev/\.git-blame-ignore-revs$'
```

## Use as a GitHub Action

[TODO](https://github.com/berquist/validate-git-blame-ignore-revs/issues/2)
