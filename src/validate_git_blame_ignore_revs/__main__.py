import argparse
from enum import Enum
from pathlib import Path
from sys import exit

from validate_git_blame_ignore_revs.lib import validate_git_blame_ignore_revs


class ErrorCode(Enum):
    FileNotFound = 0b1
    SyntaxProblem = 0b10
    CommitsNotPresent = 0b100
    MissingComments = 0b1000
    MissingCommitMessageComments = 0b10000
    MissingPreCommitCICommits = 0b100000


def main() -> None:
    parser = argparse.ArgumentParser(description="Validate a .git-blame-ignore-revs file.")
    parser.add_argument("file_path", type=Path, help="Path to the .git-blame-ignore-revs file.")
    parser.add_argument(
        "--call-git",
        action="store_true",
        help="Ensure each commit is in the history of the checked-out branch.",
    )
    parser.add_argument(
        "--strict-comments",
        action="store_true",
        help="Require each commit line to have one or more comment lines above it.",
    )
    parser.add_argument(
        "--strict-comments-git",
        action="store_true",
        help="Ensure the comment above each commit matches the first part of the commit message. Requires --strict-comments and --call-git.",
    )
    parser.add_argument(
        "--pre-commit-ci",
        action="store_true",
        help="Ensure all commits authored by pre-commit-ci[bot] are present in the file. Requires --call-git.",
    )

    args = parser.parse_args()

    retval = 0

    if args.strict_comments_git and not (args.strict_comments and args.call_git):
        parser.error("--strict-comments-git requires --strict-comments and --call-git.")
    if args.pre_commit_ci and not args.call_git:
        parser.error("--pre-commit-ci requires --call-git.")

    try:
        result = validate_git_blame_ignore_revs(
            args.file_path,
            args.call_git,
            args.strict_comments,
            args.strict_comments_git,
            args.pre_commit_ci,
        )

        print("Validation Results:")
        print(f"Valid hashes ({len(result['valid_hashes'])}):")
        for line_number, hash in result["valid_hashes"].items():
            print(f"  Line {line_number}: {hash}")

        if result["errors"]:
            print(f"\nErrors ({len(result['errors'])}):")
            for line_number, line in result["errors"].items():
                print(f"  Line {line_number}: {line}")
            retval += ErrorCode.SyntaxProblem.value
        else:
            print("\nNo errors found!")

        if args.call_git:
            if result["missing_commits"]:
                print(f"\nMissing commits ({len(result['missing_commits'])}):")
                for line_number, commit in result["missing_commits"].items():
                    print(f"  Line {line_number}: {commit}")
                retval += ErrorCode.CommitsNotPresent.value
            else:
                print("\nAll commits are present in the Git history!")

        if args.strict_comments:
            if result["strict_comment_errors"]:
                print(f"\nStrict comment errors ({len(result['strict_comment_errors'])}):")
                for line_number, line in result["strict_comment_errors"].items():
                    print(f"  Line {line_number}: {line}")
                retval += ErrorCode.MissingComments.value
            else:
                print("\nAll commit lines have comments above them!")

        if args.strict_comments_git:
            if result["comment_diffs"]:
                print(f"\nComment diffs ({len(result['comment_diffs'])}):")
                for line_number, (comment, commit_message) in result["comment_diffs"].items():
                    print(f"  Line {line_number}:")
                    print(f"    Comment: {comment}")
                    print(f"    Commit message: {commit_message}")
                retval += ErrorCode.MissingCommitMessageComments.value
            else:
                print("\nAll comments match the corresponding commit messages!")

        if args.pre_commit_ci:
            if result["missing_pre_commit_ci_commits"]:
                print(
                    f"\nMissing pre-commit-ci commits ({len(result['missing_pre_commit_ci_commits'])}):"
                )
                for commit_hash, commit_message in result["missing_pre_commit_ci_commits"].items():
                    print(f"  Commit {commit_hash}: {commit_message}")
                retval += ErrorCode.MissingPreCommitCICommits.value
            else:
                print("\nAll pre-commit-ci commits are present in the file!")
    except FileNotFoundError as e:
        print(e)
        retval += ErrorCode.FileNotFound.value

    exit(retval)


if __name__ == "__main__":
    main()
