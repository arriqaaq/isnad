---
name: Always use venv for Python
description: User requires Python packages to be installed in a virtual environment, never globally
type: feedback
---

Always use a virtual environment (venv) when installing Python packages. Never run `pip install` globally.

**Why:** User preference for clean environment isolation.
**How to apply:** Create/activate a venv before any pip install. The project already has a `.venv` defined in the Makefile.
