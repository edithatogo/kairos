// Package kairoecs provides binding metadata and data types for the Rust core.
// Engine execution requires a qualified Rust bridge; no Go core is supported.
package kairoecs

import "errors"

const Version = "0.1.0"

var (
	ErrClosed              = errors.New("kairoecs: engine is closed")
	ErrEventNotFound       = errors.New("kairoecs: event not found")
	ErrNativeNotConfigured = errors.New("kairoecs: a qualified Rust native bridge is required; the Go scheduler has been retired")
)

type EventID uint64

type Event struct {
	ID        EventID
	TimeTicks int64
	Priority  int32
	Sequence  uint64
	Kind      string
}

type Stats struct {
	Now        int64
	Scheduled  uint64
	Pending    uint64
	Dispatched uint64
	Cancelled  uint64
}

// Engine is a retired host scheduler surface. Its zero value cannot execute.
// A native engine adapter has not yet been qualified for this Go package.
type Engine struct{ closed bool }

// NewEngine fails explicitly until this package has a qualified Rust bridge.
// Breaking change: callers must handle the error result.
func NewEngine() (*Engine, error) { return NewNativeEngine() }

func SelfCheck() map[string]string {
	return map[string]string{
		"package":            "kairoecs",
		"version":            Version,
		"status":             "metadata_only",
		"native":             "not-configured",
		"execution":          "rust_native_required",
		"fallback_supported": "false",
	}
}

// NativeAvailable does not infer runtime readiness from cgo or header presence.
func NativeAvailable() bool { return false }

func NewNativeEngine() (*Engine, error) { return nil, ErrNativeNotConfigured }

func (e *Engine) Close() error {
	if e != nil {
		e.closed = true
	}
	return nil
}

func (e *Engine) unavailable() error {
	if e == nil || e.closed {
		return ErrClosed
	}
	return ErrNativeNotConfigured
}

func (e *Engine) CurrentTimeTicks() (int64, error) { return 0, e.unavailable() }
func (e *Engine) ScheduleAt(timeTicks int64, priority int32, kind string) (EventID, error) {
	return 0, e.unavailable()
}
func (e *Engine) ScheduleAfter(deltaTicks int64, priority int32, kind string) (EventID, error) {
	return 0, e.unavailable()
}
func (e *Engine) CancelEvent(id EventID) error          { return e.unavailable() }
func (e *Engine) Step() (Event, bool, error)            { return Event{}, false, e.unavailable() }
func (e *Engine) RunFor(maxEvents int) ([]Event, error) { return nil, e.unavailable() }
func (e *Engine) Stats() (Stats, error)                 { return Stats{}, e.unavailable() }
