from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from dulwich import porcelain
from validate_git_blame_ignore_revs.lib import validate_git_blame_ignore_revs

if TYPE_CHECKING:
    from dulwich.repo import Repo


@pytest.fixture(scope="session")
def repo_sst_core(tmp_path_factory: pytest.TempPathFactory) -> "Repo":
    repo = porcelain.clone(
        source="https://github.com/sstsimulator/sst-core",
        target=tmp_path_factory.mktemp(basename="sst-core"),
    )
    porcelain.checkout(repo=repo, target="17224c90014c96a245146569a27b86fb2c6d1e6b")
    return repo


@pytest.mark.skip(reason="dulwich checkout has messed-up line endings")
def test_validate_git_blame_ignore_revs_basic(repo_sst_core: "Repo") -> None:
    """The default settings only check for syntactic validity of the file."""

    repo = repo_sst_core
    repo_loc = Path(repo.path)

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
