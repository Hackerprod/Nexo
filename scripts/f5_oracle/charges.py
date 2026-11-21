"""Logical F5 charges, independently transcribed from ADR §§4.2 and 7.

These counters are model state. Only published limits or fields can be compared
with the CLI; successful work and live Array buffers are not public telemetry.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Limits:
    work: int = 16_000_000
    live_cells: int = 65_536
    frames: int = 256
    storage_bytes: int = 8_388_608
    allocation_bytes: int = 67_108_864
    observations: int = 65_536
    comptime_steps: int = 100_000
    comptime_depth: int = 64
    comptime_memory_bytes: int = 1_048_576


class BudgetFault(Exception):
    def __init__(self, code: str, kind: str, limit: int, observed: int) -> None:
        super().__init__(f"{kind}: {observed} > {limit}")
        self.code = code
        self.kind = kind
        self.limit = limit
        self.observed = observed


@dataclass
class Charges:
    limits: Limits
    work: int = 0
    live_cells: int = 0
    frames: int = 0
    storage_bytes: int = 0
    allocation_bytes: int = 0
    observations: int = 0

    def tick(self, amount: int = 1) -> None:
        for _ in range(amount):
            candidate = self.work + 1
            if candidate > self.limits.work:
                raise BudgetFault("R0001", "work", self.limits.work, candidate)
            self.work = candidate

    def reserve_cells(self, amount: int) -> None:
        candidate = self.live_cells + amount
        if candidate > self.limits.live_cells:
            raise BudgetFault("R0002", "live_value_cells", self.limits.live_cells, candidate)
        self.live_cells = candidate

    def release_cells(self, amount: int) -> None:
        self.live_cells -= amount
        assert self.live_cells >= 0

    def enter_frame(self) -> None:
        candidate = self.frames + 1
        if candidate > self.limits.frames:
            raise BudgetFault("R0002", "call_frames", self.limits.frames, candidate)
        self.frames = candidate

    def leave_frame(self) -> None:
        self.frames -= 1
        assert self.frames >= 0

    def allocate_array(self, capacity: int) -> None:
        if capacity < 0:
            raise ValueError("negative capacity")
        size = 32 * capacity
        live = self.storage_bytes + size
        total = self.allocation_bytes + size
        # F5 §7.1: runtime priority, storage before accumulated allocation.
        if live > self.limits.storage_bytes:
            raise BudgetFault("R0002", "array_storage_bytes", self.limits.storage_bytes, live)
        if total > self.limits.allocation_bytes:
            raise BudgetFault("R0002", "array_allocation_bytes", self.limits.allocation_bytes, total)
        self.storage_bytes = live
        self.allocation_bytes = total

    def release_array(self, capacity: int) -> None:
        self.storage_bytes -= 32 * capacity
        assert self.storage_bytes >= 0

    def grow_array(self, length: int, capacity: int) -> int:
        if length < capacity:
            return capacity
        new_capacity = max(1, 2 * capacity)
        # Each moved handle is guarded before either buffer is changed.
        self.tick(length)
        self.allocate_array(new_capacity)
        self.release_array(capacity)
        return new_capacity

    def observe(self, emitted: int, reserved: int = 0) -> None:
        candidate = self.observations + emitted + reserved
        if candidate > self.limits.observations:
            raise BudgetFault("R0002", "observation_bytes", self.limits.observations, candidate)
        self.observations = candidate


def range_ticks(length: int, inclusive: bool) -> int:
    """F5 §7.2 loop charges excluding endpoint expressions and body."""
    if length < 0:
        raise ValueError("negative iteration count")
    intro_and_slots = 3
    if inclusive:
        return intro_and_slots + (1 if length == 0 else 4 * length - 1)
    return intro_and_slots + 3 * length + 1


def scalar_loop_total(length: int, inclusive: bool) -> int:
    """F5-T01/T02: Main entry, four endpoint ticks and unit included."""
    return 1 + 4 + range_ticks(length, inclusive) + 1


def short_circuit_total(left: bool, op: str) -> int:
    """F5-T04–T07 for a let binding of two literal Bool operands."""
    take_right = left if op == "and" else not left
    return 1 + 1 + 1 + 2 + (2 if take_right else 0) + 1 + 1
