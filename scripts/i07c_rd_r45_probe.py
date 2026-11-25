#!/usr/bin/env python3
"""Measure source-event replay (R4) and the bounded prefix ledger (R5)."""
import i07c_rd_r1_probe as probe


probe.VARIANTS = {
    "r4_event_replay": "r4replay,r1t5",
    "r4_event_replay_t5_late": "r4replay,r1t5,r5t5",
    "r5_prefix": "r5prefix,r1t5,r5t5",
    "r5_prefix_txmarks": "r5prefix,r1t5,r5t5,txmarks,txmarksproj",
}


if __name__ == "__main__":
    probe.main()
