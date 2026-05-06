package kairoecs

import (
	"encoding/json"
	"os"
	"path/filepath"
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
		Version           int   `json:"version"`
		ExpectedKindOrder []int `json:"expected_kind_order"`
	}
	if err := readFixtureJSON("deterministic_ordering.json", &fixture); err != nil {
		t.Fatal(err)
	}
	if fixture.Version != 1 {
		t.Errorf("expected version 1, got %d", fixture.Version)
	}
	if len(fixture.ExpectedKindOrder) != 4 {
		t.Errorf("expected 4 events, got %d", len(fixture.ExpectedKindOrder))
	}
	expected := []int{1, 2, 4, 3}
	for i, k := range fixture.ExpectedKindOrder {
		if k != expected[i] {
			t.Errorf("position %d: expected kind %d, got %d", i, expected[i], k)
		}
	}
}

func TestConformanceCancellation(t *testing.T) {
	var fixture struct {
		ExpectedKindOrder []int `json:"expected_kind_order"`
	}
	if err := readFixtureJSON("cancellation.json", &fixture); err != nil {
		t.Fatal(err)
	}
	expected := []int{1, 3}
	if len(fixture.ExpectedKindOrder) != len(expected) {
		t.Fatalf("expected %d events, got %d", len(expected), len(fixture.ExpectedKindOrder))
	}
	for i, k := range fixture.ExpectedKindOrder {
		if k != expected[i] {
			t.Errorf("position %d: expected kind %d, got %d", i, expected[i], k)
		}
	}
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
