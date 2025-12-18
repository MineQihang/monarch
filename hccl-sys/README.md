# hccl-sys

Rust bindings for Huawei Collective Communication Library (HCCL), part of the CANN toolkit.

## Prerequisites

- **Huawei Ascend Driver & Firmware** installed.
- **CANN Toolkit** (Compute Architecture for Neural Networks) installed.
  - Default path: `/usr/local/Ascend/ascend-toolkit/latest`
  - Ensure `libhccl.so` is available.
- **clang** (for bindgen).

## Environment Variables

- `ASCEND_HOME_PATH`: (Optional) Path to Ascend toolkit installation. Defaults to `/usr/local/Ascend/ascend-toolkit/latest`.

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
hccl-sys = { path = "path/to/hccl-sys" }
```

## Runtime Linking

This crate dynamically loads `libhccl.so` at runtime using `dlopen`. Ensure your `LD_LIBRARY_PATH` includes the directory containing `libhccl.so` (usually included in `source /usr/local/Ascend/ascend-toolkit/set_env.sh`).

## Testing

To run tests (requires Ascend environment):

```bash
cargo test
```

