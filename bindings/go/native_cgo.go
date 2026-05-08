//go:build cgo

package kairoecs

/*
#cgo CFLAGS: -I${SRCDIR}/../../include
#include "kairo_ecs.h"
*/
import "C"

type NativeHeaderInfo struct {
	CgoEnabled      bool
	OKStatus        int
	NotFoundStatus  int
	StatsSizeBytes  uintptr
	BufferSizeBytes uintptr
}

func NativeHeaderSmoke() NativeHeaderInfo {
	return NativeHeaderInfo{
		CgoEnabled:      true,
		OKStatus:        int(C.KAIRO_ECS_OK),
		NotFoundStatus:  int(C.KAIRO_ECS_ERR_NOT_FOUND),
		StatsSizeBytes:  uintptr(C.sizeof_KairoEcsStats),
		BufferSizeBytes: uintptr(C.sizeof_KairoEcsBuffer),
	}
}
