//go:build !cgo

package kairoecs

type NativeHeaderInfo struct {
	CgoEnabled      bool
	OKStatus        int
	NotFoundStatus  int
	StatsSizeBytes  uintptr
	BufferSizeBytes uintptr
}

func NativeHeaderSmoke() NativeHeaderInfo {
	return NativeHeaderInfo{}
}
