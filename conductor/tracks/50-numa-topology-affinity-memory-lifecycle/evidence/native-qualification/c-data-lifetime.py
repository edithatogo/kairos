"""Import the real C stream batch into an installed PyArrow consumer."""

import ctypes
import gc
import struct
import sys
from pathlib import Path

import pyarrow as pa


class ArrowSchema(ctypes.Structure):
    _fields_ = [
        ("format", ctypes.c_void_p),
        ("name", ctypes.c_void_p),
        ("metadata", ctypes.c_void_p),
        ("flags", ctypes.c_int64),
        ("n_children", ctypes.c_int64),
        ("children", ctypes.c_void_p),
        ("dictionary", ctypes.c_void_p),
        ("release", ctypes.c_void_p),
        ("private_data", ctypes.c_void_p),
    ]


class ArrowArray(ctypes.Structure):
    _fields_ = [
        (field, ctypes.c_int64)
        for field in ("length", "null_count", "offset", "n_buffers", "n_children")
    ] + [
        (field, ctypes.c_void_p)
        for field in ("buffers", "children", "dictionary", "release", "private_data")
    ]


library = ctypes.CDLL(str(Path(sys.argv[1]).resolve(strict=True)))
library.track50_export_batch.argtypes = [ctypes.POINTER(ArrowSchema), ctypes.POINTER(ArrowArray)]
library.track50_export_batch.restype = ctypes.c_size_t
library.track50_release_count.argtypes = []
library.track50_release_count.restype = ctypes.c_uint
schema, array = ArrowSchema(), ArrowArray()
source_address = library.track50_export_batch(ctypes.byref(schema), ctypes.byref(array))
assert source_address and library.track50_release_count() == 0

batch = pa.RecordBatch._import_from_c(ctypes.addressof(array), ctypes.addressof(schema))
assert array.release is None and schema.release is None
field = batch.column("time_ticks_lo")
buffer = field.buffers()[1]
assert field.to_pylist() == [10, 20]
assert buffer.address == source_address
del field, batch
gc.collect()
assert library.track50_release_count() == 0
assert struct.unpack("=QQ", buffer) == (10, 20)
del buffer
gc.collect()
assert library.track50_release_count() == 1
print(f"CPython {sys.version.split()[0]}, PyArrow {pa.__version__}")
print("Imported the original C buffer, retained ticks [10, 20], and released exactly once")
