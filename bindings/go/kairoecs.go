package kairoecs

import (
	"container/heap"
	"errors"
	"fmt"
)

const Version = "0.1.0"

var (
	ErrClosed              = errors.New("kairoecs: engine is closed")
	ErrEventNotFound       = errors.New("kairoecs: event not found")
	ErrNativeNotConfigured = errors.New("kairoecs: native cgo FFI is not configured")
)

type EventID uint64

type Event struct {
	ID        EventID
	TimeTicks int64
	Priority  int32
	Sequence  uint64
	Kind      string
}

type Engine struct {
	now       int64
	nextID    EventID
	nextSeq   uint64
	closed    bool
	cancelled map[EventID]struct{}
	events    eventHeap
}

func NewEngine() *Engine {
	return &Engine{
		nextID:    1,
		cancelled: make(map[EventID]struct{}),
	}
}

func SelfCheck() map[string]string {
	nativeStatus := "not-configured"
	if NativeAvailable() {
		nativeStatus = "available"
	}

	return map[string]string{
		"package": "kairoecs",
		"version": Version,
		"status":  "ok",
		"native":  nativeStatus,
	}
}

func NativeAvailable() bool {
	return false
}

func NewNativeEngine() (*Engine, error) {
	return nil, ErrNativeNotConfigured
}

func (e *Engine) Close() error {
	if e == nil || e.closed {
		return nil
	}
	e.closed = true
	e.cancelled = nil
	e.events = nil
	return nil
}

func (e *Engine) CurrentTimeTicks() (int64, error) {
	if err := e.ensureOpen(); err != nil {
		return 0, err
	}
	return e.now, nil
}

func (e *Engine) ScheduleAt(timeTicks int64, priority int32, kind string) (EventID, error) {
	if err := e.ensureOpen(); err != nil {
		return 0, err
	}
	if timeTicks < e.now {
		return 0, fmt.Errorf("kairoecs: cannot schedule event at %d before current time %d", timeTicks, e.now)
	}

	id := e.nextID
	e.nextID++
	evt := Event{
		ID:        id,
		TimeTicks: timeTicks,
		Priority:  priority,
		Sequence:  e.nextSeq,
		Kind:      kind,
	}
	e.nextSeq++
	heap.Push(&e.events, evt)
	return id, nil
}

func (e *Engine) ScheduleAfter(deltaTicks int64, priority int32, kind string) (EventID, error) {
	if err := e.ensureOpen(); err != nil {
		return 0, err
	}
	if deltaTicks < 0 {
		return 0, fmt.Errorf("kairoecs: negative duration %d", deltaTicks)
	}
	return e.ScheduleAt(e.now+deltaTicks, priority, kind)
}

func (e *Engine) CancelEvent(id EventID) error {
	if err := e.ensureOpen(); err != nil {
		return err
	}
	if id == 0 || id >= e.nextID {
		return ErrEventNotFound
	}
	if _, ok := e.cancelled[id]; ok {
		return ErrEventNotFound
	}
	foundPending := false
	for _, evt := range e.events {
		if evt.ID == id {
			foundPending = true
			break
		}
	}
	if !foundPending {
		return ErrEventNotFound
	}
	e.cancelled[id] = struct{}{}
	return nil
}

func (e *Engine) Step() (Event, bool, error) {
	if err := e.ensureOpen(); err != nil {
		return Event{}, false, err
	}
	for e.events.Len() > 0 {
		evt := heap.Pop(&e.events).(Event)
		if _, ok := e.cancelled[evt.ID]; ok {
			delete(e.cancelled, evt.ID)
			continue
		}
		e.now = evt.TimeTicks
		return evt, true, nil
	}
	return Event{}, false, nil
}

func (e *Engine) RunFor(maxEvents int) ([]Event, error) {
	if err := e.ensureOpen(); err != nil {
		return nil, err
	}
	if maxEvents < 0 {
		return nil, fmt.Errorf("kairoecs: negative max events %d", maxEvents)
	}

	events := make([]Event, 0, maxEvents)
	for len(events) < maxEvents {
		evt, ok, err := e.Step()
		if err != nil {
			return nil, err
		}
		if !ok {
			break
		}
		events = append(events, evt)
	}
	return events, nil
}

func (e *Engine) ensureOpen() error {
	if e == nil || e.closed {
		return ErrClosed
	}
	return nil
}

type eventHeap []Event

func (h eventHeap) Len() int {
	return len(h)
}

func (h eventHeap) Less(i, j int) bool {
	if h[i].TimeTicks != h[j].TimeTicks {
		return h[i].TimeTicks < h[j].TimeTicks
	}
	if h[i].Priority != h[j].Priority {
		return h[i].Priority < h[j].Priority
	}
	return h[i].Sequence < h[j].Sequence
}

func (h eventHeap) Swap(i, j int) {
	h[i], h[j] = h[j], h[i]
}

func (h *eventHeap) Push(x any) {
	*h = append(*h, x.(Event))
}

func (h *eventHeap) Pop() any {
	old := *h
	n := len(old)
	evt := old[n-1]
	*h = old[:n-1]
	return evt
}
