#!/usr/bin/env python3
"""Measure disposable bidirectional, atomic completion links."""
import i07c_rd_r1_probe as probe

probe.VARIANTS = {
    "enlace_transaccional": "r2view,r3txlink",
    "enlace_transaccional_t5": "r2view,r3txlink,r1t5",
}

if __name__ == "__main__":
    probe.main()
