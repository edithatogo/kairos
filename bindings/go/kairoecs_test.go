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

	if _, err := engine.ScheduleAt(10, 0, "late"); err != nil {
		t.Fatal(err)
	}
	if _, err := engine.ScheduleAt(5, 10, "same-time-lower-priority"); err != nil {
		t.Fatal(err)
	}
	if _, err := engine.ScheduleAt(5, -1, "same-time-higher-priority"); err != nil {
		t.Fatal(err)
	}
	if _, err := engine.ScheduleAt(5, 10, "same-time-lower-priority-sequence"); err != nil {
		t.Fatal(err)
	}

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

	cancelled, err := engine.ScheduleAt(1, 0, "cancelled")
	if err != nil {
		t.Fatal(err)
	}
	if _, err := engine.ScheduleAt(2, 0, "kept"); err != nil {
		t.Fatal(err)
	}
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

	dispatched, err := engine.ScheduleAt(1, 0, "dispatched")
	if err != nil {
		t.Fatal(err)
	}
	cancelled, err := engine.ScheduleAt(2, 0, "cancelled")
	if err != nil {
		t.Fatal(err)
	}

	if err := engine.CancelEvent(EventID(999)); !errors.Is(err, ErrEventNotFound) {
		t.Fatalf("unexpected unknown cancellation error: %v", err)
	}
	if err := engine.CancelEvent(cancelled); err != nil {
		t.Fatalf("expected first cancellation to succeed: %v", err)
	}
	if err := engine.CancelEvent(cancelled); !errors.Is(err, ErrEventNotFound) {
		t.Fatalf("unexpected duplicate cancellation error: %v", err)
	}
	if _, ok, err := engine.Step(); err != nil || !ok {
		t.Fatalf("expected dispatched event, ok=%v err=%v", ok, err)
	}
	if err := engine.CancelEvent(dispatched); !errors.Is(err, ErrEventNotFound) {
		t.Fatalf("unexpected dispatched cancellation error: %v", err)
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

func eventKinds(events []Event) []string {
	kinds := make([]string, len(events))
	for i, evt := range events {
		kinds[i] = evt.Kind
	}
	return kinds
}
