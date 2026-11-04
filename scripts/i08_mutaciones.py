#!/usr/bin/env python3
"""Población I08 de código de producción, sobre el motor compartido de I07c."""

from pathlib import Path

import i07c_mutaciones as engine

engine.BASE = "ce3f1da"
engine.OUTPUT = Path(__file__).resolve().parents[1] / "docs/validation/i08-mutations/population.json"
engine.DIRECTORIES = ("src/check/", "src/ir/", "src/interp/", "src/diagnostic.rs", "src/main.rs")
engine.DEFAULT_ONLY = "production"
engine.TARGETED_TEST = "i08"


if __name__ == "__main__":
    engine.main()
