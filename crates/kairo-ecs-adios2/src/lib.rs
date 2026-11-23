//! Serial BP5 I/O through an installed ADIOS2 C library. The caller verifies
//! the trusted checkpoint directory and publishes complete generations.
#![cfg(all(feature = "native", any(target_os = "linux", target_os = "macos")))]
#![deny(unsafe_op_in_unsafe_fn)]

use std::{
    ffi::{c_char, c_float, c_int, c_void, CStr, CString},
    io,
    path::Path,
    ptr::NonNull,
    sync::{Mutex, MutexGuard},
};

type Handle = NonNull<c_void>;

// The types and constants below follow the ADIOS2 2.12 C API headers.
const WRITE: c_int = 1;
const READ: c_int = 2;
const SYNC: c_int = 5;
const APPEND_STEP: c_int = 0;
const READ_STEP: c_int = 2;
const STEP_OK: c_int = 0;
const END_OF_STREAM: c_int = 2;
const CONSTANT_DIMS: c_int = 1;
const GLOBAL_ARRAY: c_int = 1;
static SERIAL_IO: Mutex<()> = Mutex::new(());

extern "C" {
    fn adios2_init_serial() -> Option<Handle>;
    fn adios2_finalize(handle: Handle) -> c_int;
    fn adios2_declare_io(handle: Handle, name: *const c_char) -> Option<Handle>;
    fn adios2_set_engine(io: Handle, name: *const c_char) -> c_int;
    fn adios2_define_variable(
        io: Handle,
        name: *const c_char,
        kind: c_int,
        rank: usize,
        shape: *const usize,
        start: *const usize,
        count: *const usize,
        constant: c_int,
    ) -> Option<Handle>;
    fn adios2_open(io: Handle, path: *const c_char, mode: c_int) -> Option<Handle>;
    fn adios2_begin_step(
        engine: Handle,
        mode: c_int,
        timeout: c_float,
        status: *mut c_int,
    ) -> c_int;
    fn adios2_end_step(engine: Handle) -> c_int;
    fn adios2_close(engine: Handle) -> c_int;
    fn adios2_put(engine: Handle, var: Handle, data: *const c_void, mode: c_int) -> c_int;
    fn adios2_inquire_variable(io: Handle, name: *const c_char) -> Option<Handle>;
    fn adios2_variable_type(kind: *mut c_int, var: Handle) -> c_int;
    fn adios2_variable_shapeid(shape: *mut c_int, var: Handle) -> c_int;
    fn adios2_variable_ndims(rank: *mut usize, var: Handle) -> c_int;
    fn adios2_variable_shape(shape: *mut usize, var: Handle) -> c_int;
    fn adios2_set_selection(
        var: Handle,
        rank: usize,
        start: *const usize,
        count: *const usize,
    ) -> c_int;
    fn adios2_get(engine: Handle, var: Handle, data: *mut c_void, mode: c_int) -> c_int;
}

mod sealed {
    pub trait Sealed {}
}

/// An integer type accepted by the checkpoint columns.
pub trait Element: sealed::Sealed + Copy + Default {
    #[doc(hidden)]
    const NATIVE_TYPE: c_int;
}

macro_rules! elements {
    ($($variant:ident: $ty:ty = $tag:literal),+ $(,)?) => {
        pub enum Values {
            $($variant(Vec<$ty>)),+
        }

        impl Values {
            fn parts(&self) -> (c_int, usize, *const c_void) {
                match self {
                    $(Self::$variant(values) => ($tag, values.len(), values.as_ptr().cast())),+
                }
            }
        }

        $(
            impl From<Vec<$ty>> for Values {
                fn from(values: Vec<$ty>) -> Self {
                    Self::$variant(values)
                }
            }

            impl sealed::Sealed for $ty {}
            impl Element for $ty {
                const NATIVE_TYPE: c_int = $tag;
            }
        )+
    };
}

elements!(Bytes: u8 = 9, U32: u32 = 11, U64: u64 = 12, I32: i32 = 7);

fn checked(status: c_int, operation: &str) -> io::Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "ADIOS2 {operation} failed ({status})"
        )))
    }
}

fn handle(value: Option<Handle>, operation: &str) -> io::Result<Handle> {
    value.ok_or_else(|| io::Error::other(format!("ADIOS2 {operation} returned no handle")))
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

struct Context(Handle);

impl Context {
    fn new() -> io::Result<Self> {
        // SAFETY: This is the ownership entry point of the linked serial C API.
        Ok(Self(handle(
            unsafe { adios2_init_serial() },
            "initialization",
        )?))
    }

    fn bp5(&self) -> io::Result<Handle> {
        // SAFETY: The context remains live and the static strings terminate in NUL.
        let io = handle(
            unsafe { adios2_declare_io(self.0, c"kairos".as_ptr()) },
            "declare IO",
        )?;
        checked(
            unsafe { adios2_set_engine(io, c"BP5".as_ptr()) },
            "select BP5",
        )?;
        Ok(io)
    }

    fn finish(self) -> io::Result<()> {
        let value = self.0;
        // SAFETY: Engine cleanup has completed. Transfer finalization here.
        std::mem::forget(self);
        checked(unsafe { adios2_finalize(value) }, "finalize")
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: The only native owner of this handle is the context.
        unsafe { adios2_finalize(self.0) };
    }
}

struct Engine<'a> {
    live: Option<Handle>,
    _context: &'a Context,
}

impl<'a> Engine<'a> {
    fn open(context: &'a Context, io: Handle, path: &Path, mode: c_int) -> io::Result<Self> {
        let utf8 = path
            .to_str()
            .ok_or_else(|| invalid("ADIOS2 paths require UTF-8"))?;
        let path = CString::new(utf8).map_err(|_| invalid("ADIOS2 paths cannot contain NUL"))?;
        // SAFETY: IO belongs to the live context. Open consumes the path string
        // during this call. The engine is closed before finalizing its context.
        let engine = handle(unsafe { adios2_open(io, path.as_ptr(), mode) }, "open BP5")?;
        Ok(Self {
            live: Some(engine),
            _context: context,
        })
    }

    fn get(&self) -> Handle {
        self.live.expect("live ADIOS2 engine")
    }

    fn begin(&self, mode: c_int, expected: c_int) -> io::Result<()> {
        let mut status = -1;
        // SAFETY: The output address is valid throughout the native call.
        checked(
            unsafe { adios2_begin_step(self.get(), mode, 0.0, &mut status) },
            "begin step",
        )?;
        if status != expected {
            return Err(invalid("unexpected number or status of BP5 steps"));
        }
        Ok(())
    }

    fn end(&self) -> io::Result<()> {
        // SAFETY: All caller buffers use synchronous transfers.
        checked(unsafe { adios2_end_step(self.get()) }, "end step")
    }

    fn close(&mut self) -> io::Result<()> {
        if let Some(engine) = self.live.take() {
            // SAFETY: Take consumes ownership of this live engine exactly once.
            checked(unsafe { adios2_close(engine) }, "close")?;
        }
        Ok(())
    }
}

impl Drop for Engine<'_> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Holds exclusive access to the serial provider and its publication steps.
pub struct Session {
    _guard: MutexGuard<'static, ()>,
}

impl Session {
    pub fn acquire() -> io::Result<Self> {
        Ok(Self {
            _guard: SERIAL_IO
                .lock()
                .map_err(|_| io::Error::other("ADIOS2 session lock poisoned"))?,
        })
    }

    /// Write typed one-dimensional global arrays in one BP5 step. Empty arrays
    /// are omitted and must be declared empty by the caller's schema.
    pub fn write(&self, path: &Path, columns: &[(&CStr, Values)]) -> io::Result<()> {
        let context = Context::new()?;
        let io = context.bp5()?;
        let mut variables = Vec::with_capacity(columns.len());
        for (name, values) in columns {
            let (kind, len, data) = values.parts();
            if len > 0 {
                let start = 0;
                // SAFETY: This is a rank-one global array. The API copies both
                // the array description and the C string synchronously.
                let variable = handle(
                    unsafe {
                        adios2_define_variable(
                            io,
                            name.as_ptr(),
                            kind,
                            1,
                            &len,
                            &start,
                            &len,
                            CONSTANT_DIMS,
                        )
                    },
                    "define variable",
                )?;
                variables.push((variable, data));
            }
        }
        let mut engine = Engine::open(&context, io, path, WRITE)?;
        engine.begin(APPEND_STEP, STEP_OK)?;
        for (variable, data) in variables {
            // SAFETY: A matching typed slice stays live via columns. A synchronous
            // put finishes reading the full declared selection before returning.
            checked(
                unsafe { adios2_put(engine.get(), variable, data, SYNC) },
                "put",
            )?;
        }
        engine.end()?;
        engine.close()?;
        drop(engine);
        context.finish()
    }
}

/// Reader for the one step selected by [Session::read].
pub struct Reader<'a> {
    io: Handle,
    engine: &'a Engine<'a>,
    data_bytes: u64,
}

impl Reader<'_> {
    /// Read an exact primitive array. A count of zero requires that its variable
    /// was omitted. Allocations are bounded by the physical BP5 data file.
    pub fn array<T: Element>(&self, name: &CStr, expected: usize) -> io::Result<Vec<T>> {
        // SAFETY: The IO handle and engine remain live throughout the selected step.
        let variable = unsafe { adios2_inquire_variable(self.io, name.as_ptr()) };
        if expected == 0 && variable.is_none() {
            return Ok(Vec::new());
        }
        let variable = variable.ok_or_else(|| invalid("required BP5 variable missing"))?;
        let mut kind = -1;
        let mut shape_id = -1;
        let mut rank = 0;
        // SAFETY: These queries write typed scalars into the provided addresses.
        unsafe {
            checked(adios2_variable_type(&mut kind, variable), "variable type")?;
            checked(
                adios2_variable_shapeid(&mut shape_id, variable),
                "shape kind",
            )?;
            checked(adios2_variable_ndims(&mut rank, variable), "rank")?;
        }
        if kind != T::NATIVE_TYPE || shape_id != GLOBAL_ARRAY || rank != 1 {
            return Err(invalid(
                "expected a rank-one BP5 global array of the declared type",
            ));
        }
        let mut count = 0;
        // SAFETY: The variable has rank one, so shape writes one usize.
        checked(
            unsafe { adios2_variable_shape(&mut count, variable) },
            "shape",
        )?;
        if count == 0 || expected != count {
            return Err(invalid("unexpected BP5 column length"));
        }
        let size = count
            .checked_mul(size_of::<T>())
            .ok_or_else(|| invalid("BP5 column size overflow"))?;
        if size as u64 > self.data_bytes {
            return Err(invalid("BP5 column exceeds its physical data file"));
        }
        let mut values = Vec::new();
        values.try_reserve_exact(count).map_err(io::Error::other)?;
        values.resize(count, T::default());
        let start = 0;
        let data: *mut T = values.as_mut_ptr();
        // SAFETY: The global array selection is fully within its declared
        // shape, and the initialized buffer has the exact type and extent.
        // The synchronous operation completes before the vector can move.
        unsafe {
            checked(
                adios2_set_selection(variable, 1, &start, &count),
                "set selection",
            )?;
            checked(
                adios2_get(self.engine.get(), variable, data.cast(), SYNC),
                "get",
            )?;
        }
        Ok(values)
    }
}

impl Session {
    /// Read through a bounded selection in a single-step BP5 checkpoint.
    pub fn read<T, E: From<io::Error>>(
        &self,
        path: &Path,
        data_bytes: u64,
        decode: impl FnOnce(&Reader<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let context = Context::new()?;
        let io = context.bp5()?;
        let mut engine = Engine::open(&context, io, path, READ)?;
        engine.begin(READ_STEP, STEP_OK)?;
        let decoded = decode(&Reader {
            io,
            engine: &engine,
            data_bytes,
        });
        let ended = engine.end();
        let value = decoded?;
        ended?;
        engine.begin(READ_STEP, END_OF_STREAM)?;
        engine.close()?;
        drop(engine);
        context.finish()?;
        Ok(value)
    }
}
