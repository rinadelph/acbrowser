"""Shared test fixtures."""

import os

import pytest


@pytest.fixture(autouse=True)
def _clean_backend_env(monkeypatch):
    """Ensure CLOAKBROWSER_BACKEND doesn't leak into tests from the host environment."""
    monkeypatch.delenv("CLOAKBROWSER_BACKEND", raising=False)
