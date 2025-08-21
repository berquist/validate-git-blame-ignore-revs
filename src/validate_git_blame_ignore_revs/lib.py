import os
import re
from pathlib import Path
from subprocess import CalledProcessError, run
from typing import TypedDict, Union

__all__ = (
    "ValidationResult",
    "run_command",
    "validate_git_blame_ignore_revs",
    "HashEntries",
)

HashEntries = dict[int, str]


class ValidationResult(TypedDict):
    valid_hashes: HashEntries
    errors: HashEntries
    missing_commits: HashEntries
    strict_comment_errors: HashEntries
    comment_diffs: dict[int, tuple[str, str]]  # Line number -> (comment, commit message)
    missing_pre_commit_ci_commits: dict[str, str]  # Commit hash -> Commit message


def run_command(command: list[str]) -> str:
    """Run a Git command and return its output."""
    result = run(
        command,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def validate_git_blame_ignore_revs(
    file_path: Union[str, Path],
    call_git: bool = False,
    strict_comments: bool = False,
    strict_comments_git: bool = False,
    pre_commit_ci: bool = False,
) -> ValidationResult:
    """
    Validates the contents of a `.git-blame-ignore-revs` file.

    Args:
        file_path (Union[str, Path]): Path to the `.git-blame-ignore-revs` file.
        call_git (bool): If True, ensures each commit is in the history of the checked-out branch.
        strict_comments (bool): If True, requires each commit line to have one or more comment lines above it.
        strict_comments_git (bool): If True, ensures the comment above each commit matches the first part of the commit message.
        pre_commit_ci (bool): If True, ensures all commits authored by `pre-commit-ci[bot]` are present in the file.

    Returns:
        ValidationResult: A dictionary containing valid hashes, errors, missing commits, strict comment errors, comment diffs, and missing pre-commit-ci commits.
    """
    valid_hashes: HashEntries = {}
    errors: HashEntries = {}
    missing_commits: HashEntries = {}
    strict_comment_errors: HashEntries = {}
    comment_diffs: dict[int, tuple[str, str]] = {}
    missing_pre_commit_ci_commits: dict[str, str] = {}

    # Regular expression for a valid Git commit hash (40 hexadecimal characters)
    commit_hash_regex = re.compile(r"^[0-9a-f]{40}$")

    file_path = Path(file_path)
    lines = file_path.read_text(encoding="utf-8").splitlines()

    # Track whether the previous lines were comments
    has_comment_above = False
    last_comment = None

    for line_number, line in enumerate(lines, start=1):
        line = line.strip()

        # Skip blank lines
        if not line:
            continue

        # Check for comments
        if line.startswith("#"):
            has_comment_above = True
            last_comment = line.lstrip("#").strip()
            continue

        # Validate the commit hash
        if commit_hash_regex.match(line):
            valid_hashes[line_number] = line

            # Check strict comments requirement
            if strict_comments and not has_comment_above:
                strict_comment_errors[line_number] = line

            # Reset comment tracking after a commit line
            has_comment_above = False
            last_comment = None
        else:
            errors[line_number] = line

    if call_git or strict_comments_git:
        os.chdir(file_path.parent)

        # Fetch commit messages and verify existence using `git show`
        for line_number, commit_hash in valid_hashes.items():
            try:
                git_output = run_command(
                    ["git", "show", "--quiet", "--pretty=format:%H %s", commit_hash]
                )
                if not git_output:
                    missing_commits[line_number] = commit_hash
                else:
                    commit_hash_from_git, commit_message = git_output.split(" ", 1)
                    if strict_comments_git:
                        last_comment = get_last_comment(lines, line_number)
                        if not commit_message.startswith(last_comment):
                            comment_diffs[line_number] = (last_comment, commit_message)
            except CalledProcessError:
                missing_commits[line_number] = commit_hash

        if pre_commit_ci:
            # Fetch all commits authored by `pre-commit-ci[bot]` in the checked-out branch
            try:
                pre_commit_ci_commits = run_command(
                    ["git", "log", "--pretty=format:%H %s", r"--author=pre-commit-ci\[bot\]"]
                ).splitlines()
                for commit_entry in pre_commit_ci_commits:
                    # Skip empty or malformed lines
                    if not commit_entry.strip():
                        continue
                    parts = commit_entry.split(" ", 1)
                    if len(parts) != 2:
                        continue
                    commit_hash, commit_message = parts
                    if commit_hash not in valid_hashes.values():
                        missing_pre_commit_ci_commits[commit_hash] = commit_message
                    elif strict_comments or strict_comments_git:
                        # Check strict comments and strict comments git for pre-commit-ci commits
                        for line_number, line in valid_hashes.items():
                            if line == commit_hash:
                                last_comment = get_last_comment(lines, line_number)
                                if strict_comments and not last_comment:
                                    strict_comment_errors[line_number] = commit_hash
                                if strict_comments_git and not commit_message.startswith(
                                    last_comment
                                ):
                                    comment_diffs[line_number] = (last_comment, commit_message)
            except CalledProcessError:
                raise RuntimeError("Failed to fetch commits authored by pre-commit-ci[bot].")

    return ValidationResult(
        valid_hashes=valid_hashes,
        errors=errors,
        missing_commits=missing_commits,
        strict_comment_errors=strict_comment_errors,
        comment_diffs=comment_diffs,
        missing_pre_commit_ci_commits=missing_pre_commit_ci_commits,
    )


def get_last_comment(lines: list[str], line_number: int) -> str:
    return lines[line_number - 2].strip().lstrip("#").strip() if line_number > 1 else ""
