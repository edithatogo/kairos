//go:build cgo

package kairoecs

import "testing"

func TestNativeHeaderSmokeCompilesStableCABI(t *testing.T) {
	info := NativeHeaderSmoke()
	if !info.CgoEnabled {
		t.Fatal("expected cgo header smoke to report cgo enabled")
	}
	if info.OKStatus != 0 {
		t.Fatalf("unexpected OK status value: %d", info.OKStatus)
	}
	if info.NotFoundStatus != 2 {
		t.Fatalf("unexpected not-found status value: %d", info.NotFoundStatus)
	}
	if info.StatsSizeBytes == 0 || info.BufferSizeBytes == 0 {
		t.Fatalf("unexpected ABI struct sizes: stats=%d buffer=%d", info.StatsSizeBytes, info.BufferSizeBytes)
	}
}
