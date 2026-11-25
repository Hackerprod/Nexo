#!/usr/bin/env python3
"""Measure disposable per-read prefix views independently of R1 marks."""
import i07c_rd_r1_probe as probe

probe.VARIANTS = {
    "vistas_lectura": "r2view",
    "vistas_lectura_t5": "r2view,r1t5",
}

if __name__ == "__main__":
    probe.main()
