# Validate `.git-blame-ignore-revs`

Validate a [file used by Git for ignoring revisions in `git blame` output](https://www.michaelheap.com/git-ignore-rev/).

By convention, this file is named `.git-blame-ignore-revs` and is located at the top level or base of the repository.

## Use as a command-line tool

TODO

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

TODO, see https://github.com/berquist/validate-git-blame-ignore-revs/issues/2
