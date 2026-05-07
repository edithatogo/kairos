package kairoecs

import (
	"errors"
	"reflect"
	"testing"
)

func TestVersion(t *testing.T) {
	if Version != "0.1.0" {
		t.Fatalf("unexpected version: %s", Version)
	}
}

func TestSelfCheck(t *testing.T) {
	got := SelfCheck()
	if got["package"] != "kairoecs" || got["version"] != "0.1.0" || got["status"] != "ok" || got["native"] != "not-configured" {
		t.Fatalf("unexpected self check: %#v", got)
	}
}

func TestSchedulerOrderingIsDeterministic(t *testing.T) {
	engine := NewEngine()
	defer engine.Close()

	mustSchedule(t, engine, 10, 0, "late")
	mustSchedule(t, engine, 5, 10, "same-time-lower-priority")
	mustSchedule(t, engine, 5, -1, "same-time-higher-priority")
	mustSchedule(t, engine, 5, 10, "same-time-lower-priority-sequence")

	got, err := engine.RunFor(4)
	if err != nil {
		t.Fatal(err)
	}
	kinds := eventKinds(got)
	want := []string{
		"same-time-higher-priority",
		"same-time-lower-priority",
		"same-time-lower-priority-sequence",
		"late",
	}
	if !reflect.DeepEqual(kinds, want) {
		t.Fatalf("unexpected dispatch order: got %v want %v", kinds, want)
	}
}

func TestCancellationSkipsEvent(t *testing.T) {
	engine := NewEngine()
	defer engine.Close()

	cancelled := mustSchedule(t, engine, 1, 0, "cancelled")
	mustSchedule(t, engine, 2, 0, "kept")
	if err := engine.CancelEvent(cancelled); err != nil {
		t.Fatal(err)
	}

	got, err := engine.RunFor(2)
	if err != nil {
		t.Fatal(err)
	}
	if kinds := eventKinds(got); !reflect.DeepEqual(kinds, []string{"kept"}) {
		t.Fatalf("unexpected events after cancellation: %v", kinds)
	}
}

func TestCancellationRejectsUnknownDuplicateAndDispatchedEvent(t *testing.T) {
	engine := NewEngine()
	defer engine.Close()

	dispatched := mustSchedule(t, engine, 1, 0, "dispatched")
	cancelled := mustSchedule(t, engine, 2, 0, "cancelled")

	if err := engine.CancelEvent(EventID(999)); !errors.Is(err, ErrEventNotFound) {
		t.Fatalf("unexpected unknown cancellation error: %v", err)
	}
	if err := engine.CancelEvent(cancelled); err != nil {
		t.Fatalf("expected first cancellation to succeed: %v", err)
	}
	if err := engine.CancelEvent(cancelled); !errors.Is(err, ErrEventNotFound) {
		t.Fatalf("unexpected duplicate cancellation error: %v", err)
	}
	if evt, ok, err := engine.Step(); err != nil || !ok || evt.ID != dispatched {
		t.Fatalf("expected dispatched event %d, got evt=%+v ok=%v err=%v", dispatched, evt, ok, err)
	}
	if err := engine.CancelEvent(dispatched); !errors.Is(err, ErrEventNotFound) {
		t.Fatalf("unexpected dispatched cancellation error: %v", err)
	}
}

func TestStatsTrackScheduledCancelledDispatchedAndPending(t *testing.T) {
	engine := NewEngine()
	defer engine.Close()

	mustSchedule(t, engine, 1, 0, "first")
	cancelled := mustSchedule(t, engine, 2, 0, "cancelled")

	stats, err := engine.Stats()
	if err != nil {
		t.Fatal(err)
	}
	if stats.Scheduled != 2 || stats.Pending != 2 || stats.Dispatched != 0 || stats.Cancelled != 0 || stats.Now != 0 {
		t.Fatalf("unexpected initial stats: %+v", stats)
	}

	if err := engine.CancelEvent(cancelled); err != nil {
		t.Fatal(err)
	}
	if _, ok, err := engine.Step(); err != nil || !ok {
		t.Fatalf("expected dispatched event, ok=%v err=%v", ok, err)
	}

	stats, err = engine.Stats()
	if err != nil {
		t.Fatal(err)
	}
	if stats.Scheduled != 2 || stats.Pending != 0 || stats.Dispatched != 1 || stats.Cancelled != 1 || stats.Now != 1 {
		t.Fatalf("unexpected final stats: %+v", stats)
	}
}

func TestNativeFFIExplicitlyNotConfigured(t *testing.T) {
	if NativeAvailable() {
		t.Fatal("native FFI unexpectedly available")
	}
	if _, err := NewNativeEngine(); !errors.Is(err, ErrNativeNotConfigured) {
		t.Fatalf("unexpected native engine error: %v", err)
	}
}

func TestCloseIsExplicitAndIdempotent(t *testing.T) {
	engine := NewEngine()
	if err := engine.Close(); err != nil {
		t.Fatal(err)
	}
	if err := engine.Close(); err != nil {
		t.Fatal(err)
	}
	if _, err := engine.ScheduleAt(1, 0, "closed"); !errors.Is(err, ErrClosed) {
		t.Fatalf("unexpected closed engine error: %v", err)
	}
}

func mustSchedule(t *testing.T, engine *Engine, timeTicks int64, priority int32, kind string) EventID {
	t.Helper()
	id, err := engine.ScheduleAt(timeTicks, priority, kind)
	if err != nil {
		t.Fatal(err)
	}
	return id
}

func eventKinds(events []Event) []string {
	kinds := make([]string, len(events))
	for i, evt := range events {
		kinds[i] = evt.Kind
	}
	return kinds
}
