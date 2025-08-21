import pytest
from validate_git_blame_ignore_revs.lib import HashEntries


@pytest.fixture
def mock_git_blame_ignore_revs_file() -> str:
    return """
# This is a comment
1234567890abcdef1234567890abcdef12345678
# Another comment
abcdef1234567890abcdef1234567890abcdef12
invalid_hash
"""


@pytest.fixture
def valid_hashes() -> HashEntries:
    return {
        2: "1234567890abcdef1234567890abcdef12345678",
        4: "abcdef1234567890abcdef1234567890abcdef12",
    }


@pytest.fixture
def lines(mock_git_blame_ignore_revs_file: str) -> list[str]:
    return mock_git_blame_ignore_revs_file.strip().split("\n")
