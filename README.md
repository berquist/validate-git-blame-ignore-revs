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

TODO

## Use as a command-line tool

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
