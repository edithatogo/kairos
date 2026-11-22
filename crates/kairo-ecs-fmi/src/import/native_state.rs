use crate::{FmiError, FmiResult};
use std::cell::Cell;
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct NativeCallState {
    pub(super) fatal: Arc<AtomicBool>,
    failed: Cell<bool>,
    pending: Cell<bool>,
}

impl NativeCallState {
    #[cfg(feature = "fmi2")]
    pub(super) fn is_pending(&self) -> bool {
        self.pending.get()
    }

    pub(super) fn can_call(&self) -> bool {
        !self.fatal.load(Ordering::Acquire) && !self.pending.get()
    }

    pub(super) fn check_liveness(&self) -> FmiResult<()> {
        if self.can_call() {
            Ok(())
        } else {
            Err(FmiError::InvalidInput(
                "FMU native calls are disabled".into(),
            ))
        }
    }

    pub(super) fn check_active(&self) -> FmiResult<()> {
        self.check_liveness()?;
        if self.failed() {
            return Err(FmiError::InvalidInput(
                "FMU needs a saved state restore after an error".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn failed(&self) -> bool {
        self.failed.get()
    }

    pub(super) fn recovered(&self) {
        self.failed.set(false);
    }

    pub(super) fn accept(&self, operation: &'static str, status: i32) -> FmiResult<()> {
        self.observe(require_success(operation, status))
    }

    pub(super) fn observe<T>(&self, result: FmiResult<T>) -> FmiResult<T> {
        if let Err(FmiError::FmiStatus { status, .. }) = &result {
            match status {
                0..=2 => (),
                3 => self.failed.set(true),
                5 => self.pending.set(true), // This importer supports synchronous FMI 2 steps.
                _ => self.fatal.store(true, Ordering::Release),
            }
        }
        result
    }
}

type GetState = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32;
type SetState = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
type FreeState = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32;

/// One in-process checkpoint, owned by the component that created it.
pub(crate) struct NativeFmuState {
    get: GetState,
    set: SetState,
    free: FreeState,
    names: [&'static str; 3],
    saved: *mut c_void,
}

impl NativeFmuState {
    pub(crate) fn load(library: &libloading::Library, names: [&'static str; 3]) -> FmiResult<Self> {
        let missing = |error: libloading::Error| FmiError::SymbolNotFound(error.to_string());
        // The version-specific names select FMI's get, set and free signatures.
        let (get, set, free) = unsafe {
            (
                *library.get(names[0].as_bytes()).map_err(missing)?,
                *library.get(names[1].as_bytes()).map_err(missing)?,
                *library.get(names[2].as_bytes()).map_err(missing)?,
            )
        };
        Ok(Self {
            get,
            set,
            free,
            names,
            saved: ptr::null_mut(),
        })
    }

    pub(super) fn save(&mut self, instance: *mut c_void) -> FmiResult<()> {
        if !self.saved.is_null() {
            return Err(FmiError::InvalidInput(
                "discard the FMU checkpoint before capturing another".into(),
            ));
        }
        let mut saved = ptr::null_mut();
        let status = unsafe { (self.get)(instance, &mut saved) };
        require_success(self.names[0], status)?;
        if saved.is_null() {
            return Err(FmiError::NullComponent);
        }
        self.saved = saved;
        Ok(())
    }

    pub(super) fn restore(&self, instance: *mut c_void) -> FmiResult<()> {
        if self.saved.is_null() {
            return Err(FmiError::InvalidInput("FMU has no saved state".into()));
        }
        let status = unsafe { (self.set)(instance, self.saved) };
        require_success(self.names[1], status)
    }

    pub(super) fn clear(&mut self, instance: *mut c_void) -> FmiResult<()> {
        // Failure leaves the out argument and ownership indeterminate. Once
        // passed to free, the handle must never be used again.
        let mut saved = std::mem::replace(&mut self.saved, ptr::null_mut());
        if !saved.is_null() {
            let status = unsafe { (self.free)(instance, &mut saved) };
            require_success(self.names[2], status)?;
        }
        Ok(())
    }
}

fn require_success(operation: &'static str, status: i32) -> FmiResult<()> {
    match status {
        0 | 1 => Ok(()),
        _ => Err(FmiError::FmiStatus { operation, status }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Probe {
        token: u8,
        get_status: i32,
        free_status: i32,
        freed: u32,
    }

    unsafe extern "C" fn get(instance: *mut c_void, out: *mut *mut c_void) -> i32 {
        let probe = &mut *instance.cast::<Probe>();
        *out = if probe.get_status == 0 {
            ptr::from_mut(&mut probe.token).cast()
        } else {
            ptr::dangling_mut::<u8>().cast()
        };
        probe.get_status
    }

    unsafe extern "C" fn set(instance: *mut c_void, state: *mut c_void) -> i32 {
        let probe = &mut *instance.cast::<Probe>();
        i32::from(state != ptr::from_mut(&mut probe.token).cast()) * 3
    }

    unsafe extern "C" fn free(instance: *mut c_void, out: *mut *mut c_void) -> i32 {
        let probe = &mut *instance.cast::<Probe>();
        probe.freed += 1;
        *out = ptr::dangling_mut::<u8>().cast();
        probe.free_status
    }

    #[test]
    fn failed_state_outputs_are_never_restored_or_freed_again() {
        let mut probe = Probe {
            get_status: 2,
            ..Probe::default()
        };
        let mut state = NativeFmuState {
            get,
            set,
            free,
            names: ["get", "set", "free"],
            saved: ptr::null_mut(),
        };
        let instance = (&raw mut probe).cast();
        assert!(matches!(
            state.save(instance),
            Err(FmiError::FmiStatus { status: 2, .. })
        ));
        assert!(state.restore(instance).is_err());
        state.clear(instance).unwrap();
        assert_eq!(probe.freed, 0);

        probe.get_status = 0;
        state.save(instance).unwrap();
        assert!(state.save(instance).is_err());
        state.restore(instance).unwrap();
        probe.free_status = 2;
        assert!(matches!(
            state.clear(instance),
            Err(FmiError::FmiStatus { status: 2, .. })
        ));
        state.clear(instance).unwrap();
        assert_eq!(probe.freed, 1);
        assert!(state.restore(instance).is_err());
    }

    #[test]
    fn ordinary_errors_are_local_and_fatal_status_fences_siblings() {
        let first = NativeCallState::default();
        let sibling = NativeCallState {
            fatal: Arc::clone(&first.fatal),
            ..NativeCallState::default()
        };
        first.accept("warning", 1).unwrap();
        assert!(first.accept("error", 3).is_err());
        assert!(first.check_active().is_err());
        first.check_liveness().unwrap();
        sibling.check_active().unwrap();
        first.recovered();
        first.check_active().unwrap();
        assert!(first.accept("fatal", 4).is_err());
        assert!(first.check_liveness().is_err());
        assert!(sibling.check_liveness().is_err());
    }
}
