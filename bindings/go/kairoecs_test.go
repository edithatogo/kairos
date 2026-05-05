package kairoecs

import "testing"

func TestVersion(t *testing.T) {
	if Version != "0.1.0" {
		t.Fatalf("unexpected version: %s", Version)
	}
}

func TestSelfCheck(t *testing.T) {
	got := SelfCheck()
	if got["package"] != "kairoecs" || got["version"] != "0.1.0" || got["status"] != "ok" {
		t.Fatalf("unexpected self check: %#v", got)
	}
}
