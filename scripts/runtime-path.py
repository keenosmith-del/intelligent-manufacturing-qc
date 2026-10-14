"""Print the installed Python wheel's shared ONNX Runtime library for Rust."""

from pathlib import Path
import onnxruntime

folder = Path(onnxruntime.__file__).parent / "capi"
paths = sorted(folder.glob("libonnxruntime*.dylib")) or sorted(
    folder.glob("libonnxruntime.so*")
)
if not paths:
    raise SystemExit("ONNX Runtime shared library missing; install ml/requirements.txt")
print(paths[0].resolve())
