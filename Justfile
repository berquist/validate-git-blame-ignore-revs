# Run all project tests
test:
    uv run --directory {{justfile_directory()}} python -m pytest

# Update all project dependencies
sync:
    uv sync --directory {{justfile_directory()}} --all-extras

# Run all pre-commit checks on all files
pre:
    prek run -a
