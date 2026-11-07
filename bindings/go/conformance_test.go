package kairoecs

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// These checks only validate shared fixture availability. They are not runtime
// conformance evidence; the retired Go engine must never execute these fixtures.
func TestSharedFixtureMetadata(t *testing.T) {
	for _, name := range []string{"deterministic_ordering.json", "cancellation.json", "zero_delay_guard.json", "rng_replay.json"} {
		t.Run(name, func(t *testing.T) {
			data, err := os.ReadFile(filepath.Join("..", "..", "conformance", "fixtures", name))
			if err != nil {
				t.Fatal(err)
			}
			var fixture map[string]interface{}
			if err := json.Unmarshal(data, &fixture); err != nil {
				t.Fatal(err)
			}
			if len(fixture) == 0 {
				t.Fatal("empty fixture metadata")
			}
		})
	}
}
