# Run all project tests
test:
    uv run --project {{justfile_directory()}} python -m pytest --cov

run:
    uv run --project {{justfile_directory()}} validate-git-blame-ignore-revs --help

# Update all project dependencies
sync:
    uv sync --project {{justfile_directory()}} --all-extras

# Run all pre-commit checks on all files
pre:
    prek run -a
