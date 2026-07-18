import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))

TESTWORLD = REPO_ROOT / "tests" / "fixtures" / "testworld"
AINCRAD = REPO_ROOT / "worlds" / "aincrad"


@pytest.fixture
def testworld_path() -> Path:
    return TESTWORLD


@pytest.fixture
def aincrad_path() -> Path:
    return AINCRAD
