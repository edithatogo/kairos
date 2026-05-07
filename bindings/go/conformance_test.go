package kairoecs

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func fixturePath(name string) string {
	return filepath.Join("..", "..", "conformance", "fixtures", name)
}

func readFixtureJSON(name string, v interface{}) error {
	data, err := os.ReadFile(fixturePath(name))
	if err != nil {
		return err
	}
	return json.Unmarshal(data, v)
}

func TestConformanceDeterministicOrdering(t *testing.T) {
	var fixture struct {
		Version int `json:"version"`
		Events  []struct {
			AtTicks  int64 `json:"at_ticks"`
			Priority int32 `json:"priority"`
			Kind     int   `json:"kind"`
		} `json:"events"`
		ExpectedKindOrder []int `json:"expected_kind_order"`
	}
	if err := readFixtureJSON("deterministic_ordering.json", &fixture); err != nil {
		t.Fatal(err)
	}
	if fixture.Version != 1 {
		t.Errorf("expected version 1, got %d", fixture.Version)
	}
	engine := NewEngine()
	defer engine.Close()
	for _, event := range fixture.Events {
		mustSchedule(t, engine, event.AtTicks, event.Priority, fmt.Sprint(event.Kind))
	}
	got, err := engine.RunFor(len(fixture.Events))
	if err != nil {
		t.Fatal(err)
	}
	expected := intKindsToStrings(fixture.ExpectedKindOrder)
	if kinds := eventKinds(got); !reflect.DeepEqual(kinds, expected) {
		t.Fatalf("unexpected fixture dispatch order: got %v want %v", kinds, expected)
	}
}

func TestConformanceCancellation(t *testing.T) {
	var fixture struct {
		Events []struct {
			AtTicks  int64 `json:"at_ticks"`
			Priority int32 `json:"priority"`
			Kind     int   `json:"kind"`
			Cancel   bool  `json:"cancel"`
		} `json:"events"`
		ExpectedKindOrder []int `json:"expected_kind_order"`
	}
	if err := readFixtureJSON("cancellation.json", &fixture); err != nil {
		t.Fatal(err)
	}
	engine := NewEngine()
	defer engine.Close()
	for _, event := range fixture.Events {
		id := mustSchedule(t, engine, event.AtTicks, event.Priority, fmt.Sprint(event.Kind))
		if event.Cancel {
			if err := engine.CancelEvent(id); err != nil {
				t.Fatal(err)
			}
		}
	}
	got, err := engine.RunFor(len(fixture.Events))
	if err != nil {
		t.Fatal(err)
	}
	expected := intKindsToStrings(fixture.ExpectedKindOrder)
	if kinds := eventKinds(got); !reflect.DeepEqual(kinds, expected) {
		t.Fatalf("unexpected cancellation fixture order: got %v want %v", kinds, expected)
	}
}

func TestConformanceZeroDelayGuard(t *testing.T) {
	var fixture struct {
		Events []struct {
			AtTicks  int64 `json:"at_ticks"`
			Priority int32 `json:"priority"`
			Kind     int   `json:"kind"`
		} `json:"events"`
		ExpectedKindOrder []int `json:"expected_kind_order"`
	}
	if err := readFixtureJSON("zero_delay_guard.json", &fixture); err != nil {
		t.Fatal(err)
	}
	engine := NewEngine()
	defer engine.Close()
	for _, event := range fixture.Events {
		mustSchedule(t, engine, event.AtTicks, event.Priority, fmt.Sprint(event.Kind))
	}
	got, err := engine.RunFor(len(fixture.Events))
	if err != nil {
		t.Fatal(err)
	}
	expected := intKindsToStrings(fixture.ExpectedKindOrder)
	if kinds := eventKinds(got); !reflect.DeepEqual(kinds, expected) {
		t.Fatalf("unexpected zero-delay fixture order: got %v want %v", kinds, expected)
	}
}

func intKindsToStrings(kinds []int) []string {
	out := make([]string, len(kinds))
	for i, kind := range kinds {
		out[i] = fmt.Sprint(kind)
	}
	return out
}

func TestConformanceRngReplay(t *testing.T) {
	var fixture struct {
		RunSeed        int   `json:"run_seed"`
		ExpectedStream []int `json:"expected_stream"`
	}
	if err := readFixtureJSON("rng_replay.json", &fixture); err != nil {
		t.Fatal(err)
	}
	if fixture.RunSeed != 7 {
		t.Errorf("expected run_seed 7, got %d", fixture.RunSeed)
	}
	if len(fixture.ExpectedStream) != 4 {
		t.Errorf("expected 4 stream elements, got %d", len(fixture.ExpectedStream))
	}
}
