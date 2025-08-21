# Run all project tests
test:
    uv run --project {{justfile_directory()}} python -m pytest

# Update all project dependencies
sync:
    uv sync --project {{justfile_directory()}} --all-extras

# Run all pre-commit checks on all files
pre:
    pre-commit run -a
