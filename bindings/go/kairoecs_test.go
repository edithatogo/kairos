package kairoecs

import (
	"errors"
	"testing"
)

func TestMetadataDoesNotClaimRuntimeReadiness(t *testing.T) {
	got := SelfCheck()
	if Version != "0.1.0" || got["status"] != "metadata_only" || got["execution"] != "rust_native_required" || got["fallback_supported"] != "false" || NativeAvailable() {
		t.Fatalf("unexpected runtime claim: %#v", got)
	}
}

func TestEngineConstructorsRequireRust(t *testing.T) {
	for name, constructor := range map[string]func() (*Engine, error){"default": NewEngine, "native": NewNativeEngine} {
		t.Run(name, func(t *testing.T) {
			engine, err := constructor()
			if engine != nil || !errors.Is(err, ErrNativeNotConfigured) {
				t.Fatalf("expected explicit native failure, got engine=%v error=%v", engine, err)
			}
		})
	}
}

func TestZeroValueCannotBypassNativeRequirement(t *testing.T) {
	engine := &Engine{}
	_, nowErr := engine.CurrentTimeTicks()
	_, atErr := engine.ScheduleAt(1, 0, "event")
	_, afterErr := engine.ScheduleAfter(1, 0, "event")
	cancelErr := engine.CancelEvent(1)
	_, dispatched, stepErr := engine.Step()
	events, runErr := engine.RunFor(1)
	_, statsErr := engine.Stats()
	for _, err := range []error{nowErr, atErr, afterErr, cancelErr, stepErr, runErr, statsErr} {
		if !errors.Is(err, ErrNativeNotConfigured) {
			t.Fatalf("unexpected runtime success: %v", err)
		}
	}
	if dispatched || events != nil {
		t.Fatal("unsupported engine produced results")
	}
	if err := engine.Close(); err != nil {
		t.Fatal(err)
	}
	if err := engine.Close(); err != nil {
		t.Fatal(err)
	}
	if _, _, err := engine.Step(); !errors.Is(err, ErrClosed) {
		t.Fatal(err)
	}
}
