from pathlib import Path

import pygit2
import pytest
from validate_git_blame_ignore_revs.lib import validate_git_blame_ignore_revs


@pytest.fixture(scope="session")
def repo_sst_core(tmp_path_factory: pytest.TempPathFactory) -> pygit2.Repository:
    repo = pygit2.clone_repository(
        url="https://github.com/sstsimulator/sst-core.git",
        path=str(tmp_path_factory.mktemp(basename="sst-core")),
    )
    repo.reset("17224c90014c96a245146569a27b86fb2c6d1e6b", pygit2.enums.ResetMode.HARD)
    return repo


def test_validate_git_blame_ignore_revs_basic(repo_sst_core: pygit2.Repository) -> None:
    """The default settings only check for syntactic validity of the file."""

    repo = repo_sst_core
    repo_loc = Path(repo.workdir)

    result = validate_git_blame_ignore_revs(
        file_path=repo_loc / ".git-blame-ignore-revs",
        call_git=False,
        strict_comments=False,
        strict_comments_git=False,
        pre_commit_ci=False,
    )

    assert result["valid_hashes"]
    assert not result["errors"]
    assert not result["missing_commits"]
    assert not result["strict_comment_errors"]
    assert not result["comment_diffs"]
    assert not result["missing_pre_commit_ci_commits"]

    assert result["valid_hashes"] == {
        2: "f150d6f7db2d71f4a77720ee7e7a7385a57bf540",
        5: "1c149870b806f5412c34c918cea96caa14720f3c",
        8: "03ebe85afb00f521ca6f8822e48a4a17711f96dd",
        11: "89e4bf10e7e9c6a7789c66c80d57a6ead099f132",
        14: "a2b3aa4e811e5f9a763f55e5da0287c773c669c9",
        17: "3e0a31a25c55b8c3d53e454f61de81358582c6c0",
    }


def test_validate_git_blame_ignore_revs_strict_comments(repo_sst_core: pygit2.Repository) -> None:
    """In addition to valid hashes, check that each has one or more comment
    lines associated with it (above it).
    """

    repo = repo_sst_core
    repo_loc = Path(repo.workdir)

    result = validate_git_blame_ignore_revs(
        file_path=repo_loc / ".git-blame-ignore-revs",
        call_git=False,
        strict_comments=True,
        strict_comments_git=False,
        pre_commit_ci=False,
    )

    assert result["valid_hashes"]
    assert not result["errors"]
    assert not result["missing_commits"]
    assert not result["strict_comment_errors"]
    assert not result["comment_diffs"]
    assert not result["missing_pre_commit_ci_commits"]

    assert result["valid_hashes"] == {
        2: "f150d6f7db2d71f4a77720ee7e7a7385a57bf540",
        5: "1c149870b806f5412c34c918cea96caa14720f3c",
        8: "03ebe85afb00f521ca6f8822e48a4a17711f96dd",
        11: "89e4bf10e7e9c6a7789c66c80d57a6ead099f132",
        14: "a2b3aa4e811e5f9a763f55e5da0287c773c669c9",
        17: "3e0a31a25c55b8c3d53e454f61de81358582c6c0",
    }


def test_validate_git_blame_ignore_revs_call_git(repo_sst_core: pygit2.Repository) -> None:
    """For each commit hash, check that it actually exists in the Git repository."""

    repo = repo_sst_core
    repo_loc = Path(repo.workdir)

    result = validate_git_blame_ignore_revs(
        file_path=repo_loc / ".git-blame-ignore-revs",
        call_git=True,
        strict_comments=False,
        strict_comments_git=False,
        pre_commit_ci=False,
    )

    assert result["valid_hashes"]
    assert not result["errors"]
    assert result["missing_commits"]
    assert not result["strict_comment_errors"]
    assert not result["comment_diffs"]
    assert not result["missing_pre_commit_ci_commits"]

    assert result["valid_hashes"] == {
        2: "f150d6f7db2d71f4a77720ee7e7a7385a57bf540",
        5: "1c149870b806f5412c34c918cea96caa14720f3c",
        8: "03ebe85afb00f521ca6f8822e48a4a17711f96dd",
        11: "89e4bf10e7e9c6a7789c66c80d57a6ead099f132",
        14: "a2b3aa4e811e5f9a763f55e5da0287c773c669c9",
        17: "3e0a31a25c55b8c3d53e454f61de81358582c6c0",
    }

    assert result["missing_commits"] == {
        2: "f150d6f7db2d71f4a77720ee7e7a7385a57bf540",
        8: "03ebe85afb00f521ca6f8822e48a4a17711f96dd",
    }


def test_validate_git_blame_ignore_revs_strict_comments_git(
    repo_sst_core: pygit2.Repository,
) -> None:
    """For each commit hash, check that its comment matches the commit
    subject.
    """

    repo = repo_sst_core
    repo_loc = Path(repo.workdir)

    result = validate_git_blame_ignore_revs(
        file_path=repo_loc / ".git-blame-ignore-revs",
        call_git=True,
        strict_comments=True,
        strict_comments_git=True,
        pre_commit_ci=False,
    )

    assert result["valid_hashes"]
    assert not result["errors"]
    assert result["missing_commits"]
    assert not result["strict_comment_errors"]
    assert result["comment_diffs"]
    assert not result["missing_pre_commit_ci_commits"]

    assert result["valid_hashes"] == {
        2: "f150d6f7db2d71f4a77720ee7e7a7385a57bf540",
        5: "1c149870b806f5412c34c918cea96caa14720f3c",
        8: "03ebe85afb00f521ca6f8822e48a4a17711f96dd",
        11: "89e4bf10e7e9c6a7789c66c80d57a6ead099f132",
        14: "a2b3aa4e811e5f9a763f55e5da0287c773c669c9",
        17: "3e0a31a25c55b8c3d53e454f61de81358582c6c0",
    }

    assert result["missing_commits"] == {
        2: "f150d6f7db2d71f4a77720ee7e7a7385a57bf540",
        8: "03ebe85afb00f521ca6f8822e48a4a17711f96dd",
    }

    assert result["comment_diffs"] == {
        5: ("Initial cmake-format fixes", "Apply cmake-format fixes"),
        17: ("Move to clang-format v20", "Update to clang-format v20. (#1322)"),
    }


def test_validate_git_blame_ignore_revs_all(
    repo_sst_core: pygit2.Repository,
) -> None:
    """Perform all checks."""

    repo = repo_sst_core
    repo_loc = Path(repo.workdir)

    result = validate_git_blame_ignore_revs(
        file_path=repo_loc / ".git-blame-ignore-revs",
        call_git=True,
        strict_comments=True,
        strict_comments_git=True,
        pre_commit_ci=True,
    )

    assert result["valid_hashes"]
    assert not result["errors"]
    assert result["missing_commits"]
    assert not result["strict_comment_errors"]
    assert result["comment_diffs"]
    # This repository doesn't have any automated pre-commit.ci commits.
    assert not result["missing_pre_commit_ci_commits"]

    assert result["valid_hashes"] == {
        2: "f150d6f7db2d71f4a77720ee7e7a7385a57bf540",
        5: "1c149870b806f5412c34c918cea96caa14720f3c",
        8: "03ebe85afb00f521ca6f8822e48a4a17711f96dd",
        11: "89e4bf10e7e9c6a7789c66c80d57a6ead099f132",
        14: "a2b3aa4e811e5f9a763f55e5da0287c773c669c9",
        17: "3e0a31a25c55b8c3d53e454f61de81358582c6c0",
    }

    assert result["missing_commits"] == {
        2: "f150d6f7db2d71f4a77720ee7e7a7385a57bf540",
        8: "03ebe85afb00f521ca6f8822e48a4a17711f96dd",
    }

    assert result["comment_diffs"] == {
        5: ("Initial cmake-format fixes", "Apply cmake-format fixes"),
        17: ("Move to clang-format v20", "Update to clang-format v20. (#1322)"),
    }
