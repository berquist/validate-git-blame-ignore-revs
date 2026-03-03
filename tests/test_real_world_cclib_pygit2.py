from pathlib import Path

import pygit2
import pytest
from validate_git_blame_ignore_revs.lib import validate_git_blame_ignore_revs


@pytest.fixture(scope="session")
def repo_cclib(tmp_path_factory: pytest.TempPathFactory) -> pygit2.Repository:
    repo = pygit2.clone_repository(
        url="https://github.com/cclib/cclib.git",
        path=str(tmp_path_factory.mktemp(basename="cclib")),
    )
    oid = pygit2.Oid(hex="4557cf6d8e3eafdf76e80daa4b5de0bdc4d8ec2c")
    repo.checkout_tree(treeish=repo[oid])
    repo.set_head(target=oid)
    return repo


def test_validate_git_blame_ignore_revs_basic(repo_cclib: pygit2.Repository) -> None:
    """The default settings only check for syntactic validity of the file."""

    repo = repo_cclib
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
        2: "fb35435f66eeb8b4825f7022cc2ab315e5379483",
        5: "37740d43064bc13445b19ff2d3c5f1154f202896",
        7: "8f3fbf7d1fc060a3c8522343dd103604bd946e5d",
        9: "7b2e8701dcb1a1a6c437919b185be78a35c3e2a5",
        11: "e986fd9bb37ca0113707f88f6fea7f2318671cdd",
        13: "04b3e4694e001e4b91457674a490b723e17af2b7",
        15: "3c499da3e44e4a8ae983407c0f07c71996d202d8",
        16: "06e54bd1eb663dd948973037f336d5190f4734ce",
        18: "7f407d1c7383a17cbe0995fc7bd65daf964f2eb9",
        20: "0ae04b8fd098645aa3d4805abba311ccb86762dc",
        22: "57041a7cd327d4206d9c870b7d37acaa556596c6",
        24: "8243cb2c418f4f373c3a3e48ffa0c213dd77988e",
        26: "47ee1d84e2325b15c9e5508d6d48ae1f49e507b3",
    }


def test_validate_git_blame_ignore_revs_strict_comments(repo_cclib: pygit2.Repository) -> None:
    """In addition to valid hashes, check that each has one or more comment
    lines associated with it (above it).
    """

    repo = repo_cclib
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
    assert result["strict_comment_errors"]
    assert not result["comment_diffs"]
    assert not result["missing_pre_commit_ci_commits"]

    assert result["valid_hashes"] == {
        2: "fb35435f66eeb8b4825f7022cc2ab315e5379483",
        5: "37740d43064bc13445b19ff2d3c5f1154f202896",
        7: "8f3fbf7d1fc060a3c8522343dd103604bd946e5d",
        9: "7b2e8701dcb1a1a6c437919b185be78a35c3e2a5",
        11: "e986fd9bb37ca0113707f88f6fea7f2318671cdd",
        13: "04b3e4694e001e4b91457674a490b723e17af2b7",
        15: "3c499da3e44e4a8ae983407c0f07c71996d202d8",
        16: "06e54bd1eb663dd948973037f336d5190f4734ce",
        18: "7f407d1c7383a17cbe0995fc7bd65daf964f2eb9",
        20: "0ae04b8fd098645aa3d4805abba311ccb86762dc",
        22: "57041a7cd327d4206d9c870b7d37acaa556596c6",
        24: "8243cb2c418f4f373c3a3e48ffa0c213dd77988e",
        26: "47ee1d84e2325b15c9e5508d6d48ae1f49e507b3",
    }

    # The second "isort" commit doesn't have a comment.
    assert result["strict_comment_errors"] == {
        16: "06e54bd1eb663dd948973037f336d5190f4734ce",
    }


# We don't bother with --call-git alone because all of the commit refs are
# known to be present in the repository.


def test_validate_git_blame_ignore_revs_strict_comments_git(
    repo_cclib: pygit2.Repository,
) -> None:
    """For each commit hash, check that its comment matches the commit
    subject.
    """

    repo = repo_cclib
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
    assert not result["missing_commits"]
    assert result["strict_comment_errors"]
    assert result["comment_diffs"]
    assert not result["missing_pre_commit_ci_commits"]

    assert result["valid_hashes"] == {
        2: "fb35435f66eeb8b4825f7022cc2ab315e5379483",
        5: "37740d43064bc13445b19ff2d3c5f1154f202896",
        7: "8f3fbf7d1fc060a3c8522343dd103604bd946e5d",
        9: "7b2e8701dcb1a1a6c437919b185be78a35c3e2a5",
        11: "e986fd9bb37ca0113707f88f6fea7f2318671cdd",
        13: "04b3e4694e001e4b91457674a490b723e17af2b7",
        15: "3c499da3e44e4a8ae983407c0f07c71996d202d8",
        16: "06e54bd1eb663dd948973037f336d5190f4734ce",
        18: "7f407d1c7383a17cbe0995fc7bd65daf964f2eb9",
        20: "0ae04b8fd098645aa3d4805abba311ccb86762dc",
        22: "57041a7cd327d4206d9c870b7d37acaa556596c6",
        24: "8243cb2c418f4f373c3a3e48ffa0c213dd77988e",
        26: "47ee1d84e2325b15c9e5508d6d48ae1f49e507b3",
    }

    # The second "isort" commit doesn't have a comment.
    assert result["strict_comment_errors"] == {
        16: "06e54bd1eb663dd948973037f336d5190f4734ce",
    }

    assert result["comment_diffs"] == {
        5: ("remove trailing whitespace", "pre-commit: fix trailing-whitespace"),
        7: ("fix end of file", "pre-commit: end-of-file-fixer"),
        9: ("fix line endings to be LF", "make all line endings LF"),
        11: ("ruff for all but cclib/parser", "apply ruff to all but cclib/parser"),
        13: ("ruff for cclib/parser", "apply ruff to cclib/parser"),
        15: ("isort", "apply isort"),
        # TODO fix this side effect of --strict-comments
        16: ("3c499da3e44e4a8ae983407c0f07c71996d202d8", "isort: fix for circular import"),
        18: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
        20: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
        22: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
        24: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
        26: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
    }


def test_validate_git_blame_ignore_revs_all(
    repo_cclib: pygit2.Repository,
) -> None:
    """Perform all checks."""

    repo = repo_cclib
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
    assert not result["missing_commits"]
    assert result["strict_comment_errors"]
    assert result["comment_diffs"]
    assert result["missing_pre_commit_ci_commits"]

    assert result["valid_hashes"] == {
        2: "fb35435f66eeb8b4825f7022cc2ab315e5379483",
        5: "37740d43064bc13445b19ff2d3c5f1154f202896",
        7: "8f3fbf7d1fc060a3c8522343dd103604bd946e5d",
        9: "7b2e8701dcb1a1a6c437919b185be78a35c3e2a5",
        11: "e986fd9bb37ca0113707f88f6fea7f2318671cdd",
        13: "04b3e4694e001e4b91457674a490b723e17af2b7",
        15: "3c499da3e44e4a8ae983407c0f07c71996d202d8",
        16: "06e54bd1eb663dd948973037f336d5190f4734ce",
        18: "7f407d1c7383a17cbe0995fc7bd65daf964f2eb9",
        20: "0ae04b8fd098645aa3d4805abba311ccb86762dc",
        22: "57041a7cd327d4206d9c870b7d37acaa556596c6",
        24: "8243cb2c418f4f373c3a3e48ffa0c213dd77988e",
        26: "47ee1d84e2325b15c9e5508d6d48ae1f49e507b3",
    }

    # The second "isort" commit doesn't have a comment.
    assert result["strict_comment_errors"] == {
        16: "06e54bd1eb663dd948973037f336d5190f4734ce",
    }

    assert result["comment_diffs"] == {
        5: ("remove trailing whitespace", "pre-commit: fix trailing-whitespace"),
        7: ("fix end of file", "pre-commit: end-of-file-fixer"),
        9: ("fix line endings to be LF", "make all line endings LF"),
        11: ("ruff for all but cclib/parser", "apply ruff to all but cclib/parser"),
        13: ("ruff for cclib/parser", "apply ruff to cclib/parser"),
        15: ("isort", "apply isort"),
        # TODO fix this side effect of --strict-comments
        16: ("3c499da3e44e4a8ae983407c0f07c71996d202d8", "isort: fix for circular import"),
        18: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
        20: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
        22: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
        24: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
        26: ("pre-commit autofix", "[pre-commit.ci] auto fixes from pre-commit.com hooks"),
    }

    assert result["missing_pre_commit_ci_commits"] == {
        "038bbefc86b090ed18572e35665b939e45215ea7": "[pre-commit.ci] pre-commit autoupdate",
        "089c6ee49a9c6035046979dfe665ed105b346dbc": "[pre-commit.ci] auto fixes from "
        "pre-commit.com hooks",
        "0b06cb3fd5563978fa361edc42f1a49a81ac457c": "[pre-commit.ci] auto fixes from "
        "pre-commit.com hooks",
        "0f290e88405383f4a2f9dce7b3e67e958ea0212a": "[pre-commit.ci] pre-commit autoupdate",
        "0f4d7afbe9ca65f9c4d102f92eecacc78e76db6c": "[pre-commit.ci] pre-commit autoupdate",
        "106dab76ebb1ed6e4737319ad67e0abe5f233146": "[pre-commit.ci] pre-commit autoupdate",
        "14878b2f6508a0fabb4b09fd0d09cb1db3f6b656": "[pre-commit.ci] pre-commit autoupdate",
        "15624b49b0a7ab8fab50843d1ff2e969962e6aad": "[pre-commit.ci] auto fixes from "
        "pre-commit.com hooks",
        "186817fcd59bc253a432f73a2ed36ea7679b7991": "[pre-commit.ci] pre-commit autoupdate",
        "21c3289ff45fb65746d5795edfc1515fc22822b1": "[pre-commit.ci] pre-commit autoupdate",
        "2590884a2d8a1ded2492b9bb145b19cc3434d7b7": "[pre-commit.ci] pre-commit autoupdate",
        "25ef9af951ef7195c3b26fdae7949cb76ec0cea1": "[pre-commit.ci] pre-commit autoupdate",
        "2e984315d3ea4bc9f5532e8db078a0affda428b3": "[pre-commit.ci] pre-commit autoupdate",
        "2f2262f35d7d8d18852cce48ccac0629fd630d67": "[pre-commit.ci] pre-commit autoupdate",
        "33f470c73f582bb6a0d127a115bb3ae404eb9731": "[pre-commit.ci] pre-commit autoupdate",
        "3b4b4a6dbe7dd7ffc418bdf3e4300209ad2525e0": "[pre-commit.ci] pre-commit autoupdate",
        "54bdc19452c9107cddad6090f5f9b20fb88cae44": "[pre-commit.ci] pre-commit autoupdate",
        "5a926059ffd91c555e76e4ab113324b203b2214c": "[pre-commit.ci] pre-commit autoupdate",
        "5e3393e87c8cfa80a923c6318f804e89036a69e0": "[pre-commit.ci] pre-commit autoupdate",
        "635d13f7ca0c74623356661c851768b6f3afa151": "[pre-commit.ci] pre-commit autoupdate",
        "67b0f630cb0894a3f77f1d26cb06d568a8cd1e2e": "[pre-commit.ci] pre-commit autoupdate",
        "688d3e428552100ce2bf584c8beb9c7a89d9b6d6": "[pre-commit.ci] pre-commit autoupdate",
        "79e919100300bdeddcfe27caaab55a43b6ec0719": "[pre-commit.ci] pre-commit autoupdate",
        "7cd56513313c7534523c219d6a7ef44bc010e241": "[pre-commit.ci] pre-commit autoupdate",
        "7faac65ddec1a342011c026c52bdd462569c432b": "[pre-commit.ci] pre-commit autoupdate",
        "8261954e2f0cb6e7e896e4139b18d588fe911508": "[pre-commit.ci] pre-commit autoupdate",
        "849c8292910fbb74449fa42e0f95520081c71388": "[pre-commit.ci] pre-commit autoupdate",
        "8748529bc89e35b83e1dd34431fe91b3d4ba57e5": "[pre-commit.ci] pre-commit autoupdate",
        "8b6396ad1f06ee3f777f3fdd041075bc680f2a31": "[pre-commit.ci] pre-commit autoupdate",
        "8bc987191430e5a880bc69af8decc95155ade3e2": "[pre-commit.ci] pre-commit autoupdate",
        "8ccbeb072413ca9dda3a0da88d54f4ed55ab59b7": "[pre-commit.ci] pre-commit autoupdate",
        "8e0410ac9e8b85ed17ed687227b8ca4e474234f5": "[pre-commit.ci] pre-commit autoupdate",
        "900a6eef07ad9fc4a03ebfb5de4327cd0842fb66": "[pre-commit.ci] pre-commit autoupdate",
        "93c0492f0d05f6fcc40fa5874c0f4ed38099bbff": "[pre-commit.ci] pre-commit autoupdate",
        "981fe13cfec0d7a0d22eae4c4ee276dd9face9ee": "[pre-commit.ci] pre-commit autoupdate",
        "9fd1fdc0e3cb3d85336b81fa48ddd708170bb31b": "[pre-commit.ci] auto fixes from "
        "pre-commit.com hooks",
        "a54c1eb8ce367677c5f35ab1b4f70de2b0660282": "[pre-commit.ci] pre-commit autoupdate",
        "a856068ba0b9ae46d1d6f52681b49b0297de2ca5": "[pre-commit.ci] pre-commit autoupdate",
        "adde67a3e7b5f12d1051cb41535e15e63a3f9c9b": "Update license year from 2024 to 2025",
        "b1ed6e2cd427dccc5c875dfc4b0d90d0e6e850f3": "[pre-commit.ci] auto fixes from "
        "pre-commit.com hooks",
        "b892a126f0d1d1ca37c62495a89de75feb49ccf7": "[pre-commit.ci] pre-commit autoupdate",
        "bc00b002eafcaace2024f33e64291ea37181c278": "[pre-commit.ci] pre-commit autoupdate",
        "c1d280e0fdee1473b6ee50d0c9e0e4eef609acff": "[pre-commit.ci] pre-commit autoupdate",
        "c2a633c36d67cdf83486b9bc41151490d5a60291": "[pre-commit.ci] pre-commit autoupdate",
        "c2f215794365df9c3e447bfbb0c008d6f755e139": "[pre-commit.ci] pre-commit autoupdate",
        "d08971e85461bcf32f6562d6b86eac48e652fa13": "[pre-commit.ci] pre-commit autoupdate",
        "d24f91ac92be4bd73a5271dda1a7a2c7059ec5e2": "[pre-commit.ci] pre-commit autoupdate",
        "d51a21a6a63dc368aeea8004851f9c63607b4a8c": "[pre-commit.ci] pre-commit autoupdate",
        "d8bae46f30390f43c868d0d28119998a87a7d440": "[pre-commit.ci] pre-commit autoupdate",
        "dc90f9920f43dfb8f300a35e8d3cc670c21606ac": "[pre-commit.ci] pre-commit autoupdate",
        "e1841cd47230a6c1f48c7d44e0e1ef6d7c1c12b4": "[pre-commit.ci] pre-commit autoupdate",
        "e66b13a2157fc3f071e9c03ec7b897f118753062": "[pre-commit.ci] pre-commit autoupdate",
        "ea847a6d5c9508408f0eb7a741fe5f1414e015a7": "[pre-commit.ci] pre-commit autoupdate",
        "eb166a9c655810e550ce98216bc83becb0de2404": "[pre-commit.ci] pre-commit autoupdate",
        "ebb0e7b996d2d1c8c4b037634e6311fedc9f1930": "[pre-commit.ci] pre-commit autoupdate",
        "ec118b5f4412a340a19ed2b183fd9a1bfb8e2430": "[pre-commit.ci] pre-commit autoupdate",
        "f253f860459e578e56aa10adf7edacdc2c98e778": "[pre-commit.ci] pre-commit autoupdate",
        "f508805db96c1ce60c628b62d38fbb0f4979da8f": "[pre-commit.ci] pre-commit autoupdate",
        "f7c8c8a33dd1f9ada079acdf4cf369d86632bf07": "[pre-commit.ci] pre-commit autoupdate",
        "f84e9d4ae7f757877a8d30e1feffd8394d11b213": "[pre-commit.ci] pre-commit autoupdate",
    }
