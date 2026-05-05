"""Python-native scheduler facade used before native wheels are configured."""

from __future__ import annotations

from heapq import heappop, heappush

from ._types import DispatchedEvent, EventId, ScheduleRequest, SimTime, StepOutcome


class Scheduler:
    """Deterministic single-threaded scheduler matching the core v1 order."""

    def __init__(self) -> None:
        self._heap: list[tuple[int, int, int, EventId, ScheduleRequest]] = []
        self._cancelled: set[EventId] = set()
        self._next_event_index = 0
        self._next_event_generation = 0
        self._next_sequence = 0
        self._now = SimTime.ZERO
        self._dispatched = 0
        self.trace: list[DispatchedEvent] = []

    @property
    def now(self) -> SimTime:
        return self._now

    @property
    def pending_events(self) -> int:
        return max(0, len(self._heap) - len(self._cancelled))

    @property
    def dispatched_events(self) -> int:
        return self._dispatched

    def schedule(self, request: ScheduleRequest) -> EventId:
        event_id = EventId(self._next_event_index, self._next_event_generation)
        self._next_event_index += 1
        self._next_event_generation = (self._next_event_generation + 1) & 0xFFFFFFFF
        sequence = self._next_sequence
        self._next_sequence += 1
        heappush(self._heap, (request.at.ticks, request.priority, sequence, event_id, request))
        return event_id

    def schedule_at(
        self,
        at_ticks: int,
        *,
        priority: int = 0,
        kind: int = 0,
    ) -> EventId:
        return self.schedule(ScheduleRequest(SimTime.from_ticks(at_ticks), priority, kind))

    def schedule_after(
        self,
        after_ticks: int,
        *,
        priority: int = 0,
        kind: int = 0,
    ) -> EventId:
        return self.schedule_at(self._now.ticks + after_ticks, priority=priority, kind=kind)

    def cancel(self, event_id: EventId) -> bool:
        before = len(self._cancelled)
        self._cancelled.add(event_id)
        return len(self._cancelled) != before

    def step(self) -> tuple[StepOutcome, DispatchedEvent | None]:
        while self._heap:
            _at, _priority, sequence, event_id, request = heappop(self._heap)
            if event_id in self._cancelled:
                self._cancelled.remove(event_id)
                continue

            self._now = request.at
            event = DispatchedEvent(
                id=event_id,
                at=request.at,
                priority=request.priority,
                sequence=sequence,
                kind=request.kind,
                entity=request.entity,
            )
            self._dispatched += 1
            self.trace.append(event)
            return StepOutcome.DISPATCHED, event

        return StepOutcome.EMPTY, None

    def run_for(self, max_events: int) -> tuple[StepOutcome, DispatchedEvent | None]:
        if max_events == 0:
            return StepOutcome.LIMIT_REACHED, None

        last: tuple[StepOutcome, DispatchedEvent | None] = (StepOutcome.EMPTY, None)
        for _ in range(max_events):
            outcome = self.step()
            if outcome[0] is StepOutcome.EMPTY:
                return last if last[0] is StepOutcome.DISPATCHED else outcome
            last = outcome

        return (StepOutcome.LIMIT_REACHED, last[1]) if self._heap else last

    def run_until(self, time_limit_ticks: int) -> tuple[StepOutcome, DispatchedEvent | None]:
        last: tuple[StepOutcome, DispatchedEvent | None] = (StepOutcome.EMPTY, None)
        while self._heap:
            next_time = self._heap[0][0]
            if next_time > time_limit_ticks:
                return (StepOutcome.LIMIT_REACHED, last[1])
            outcome = self.step()
            if outcome[0] is StepOutcome.EMPTY:
                return last
            last = outcome
        return last

    def stats(self) -> dict[str, int]:
        return {
            "current_time_ticks": self._now.ticks,
            "pending_events": self.pending_events,
            "dispatched_events": self._dispatched,
        }
