/*
 * Copyright (c) Huawei Technologies Co., Ltd.
 * All rights reserved.
 *
 * This source code is licensed under the BSD-style license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::path::PathBuf;
use std::env;

#[cfg(target_os = "macos")]
fn main() {}

#[cfg(not(target_os = "macos"))]
fn main() {
    // Compile the bridge.cpp file
    let mut cc_builder = cc::Build::new();
    cc_builder
        .cpp(true)
        .file("src/bridge.cpp")
        .flag("-std=c++14"); // HCCL/CANN typically uses C++11 or newer

    // Try to find Ascend/CANN home to include ACL headers if needed
    // Common paths: /usr/local/Ascend/ascend-toolkit/latest
    let ascend_home = env::var("ASCEND_HOME_PATH").ok().or_else(|| {
        let default_path = PathBuf::from("/usr/local/Ascend/ascend-toolkit/latest");
        if default_path.exists() {
            Some(default_path.to_string_lossy().to_string())
        } else {
            None
        }
    });

    if let Some(home) = &ascend_home {
        cc_builder.include(format!("{}/include", home));
        // Also add to bindgen include path later
    }

    // Include local hccl headers
    cc_builder.include("cann-hccl/inc");
    cc_builder.include("src"); // for acl_mock.h

    cc_builder.compile("hccl_bridge");

    // Bindgen setup
    let mut builder = bindgen::Builder::default()
        .header("src/bridge.h")
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-std=c++14")
        .clang_arg("-Icann-hccl/inc") // Add include path for hccl headers
        .clang_arg("-Isrc") // Add include path for acl_mock.h
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Version and error handling
        .allowlist_function("HcclGetErrorString")
        // Communicator creation and management
        .allowlist_function("HcclCommInitClusterInfo")
        .allowlist_function("HcclCommInitAll")
        .allowlist_function("HcclGetRootInfo")
        .allowlist_function("HcclCommInitRootInfo")
        .allowlist_function("HcclCommDestroy")
        .allowlist_function("HcclGetRankSize")
        .allowlist_function("HcclGetRankId")
        .allowlist_function("HcclGetCommAsyncError")
        .allowlist_function("HcclCommSuspend")
        .allowlist_function("HcclCommResume")
        // ACL functions
        .allowlist_function("aclrtSetDevice")
        .allowlist_function("aclrtStreamSynchronize")
        // Collective communication
        .allowlist_function("HcclAllReduce")
        .allowlist_function("HcclBroadcast")
        .allowlist_function("HcclAllGather")
        .allowlist_function("HcclReduceScatter")
        .allowlist_function("HcclReduce")
        .allowlist_function("HcclAlltoAll")
        .allowlist_function("HcclAlltoAllV")
        .allowlist_function("HcclBarrier")
        // Point to point communication
        .allowlist_function("HcclSend")
        .allowlist_function("HcclRecv")
        // Types
        .allowlist_type("HcclComm")
        .allowlist_type("HcclResult")
        .allowlist_type("HcclDataType")
        .allowlist_type("HcclReduceOp")
        .allowlist_type("HcclRootInfo")
        .allowlist_type("aclrtStream")
        .blocklist_type("aclrtDrvMemHandle") // We don't expose this deeply yet
        .default_enum_style(bindgen::EnumVariation::NewType {
            is_bitfield: false,
            is_global: false,
        });

    if let Some(home) = &ascend_home {
        builder = builder.clang_arg(format!("-I{}/include", home));
    }

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    builder
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Link against system libraries required for dynamic loading
    println!("cargo::rustc-link-lib=dl");
    println!("cargo::rustc-link-lib=pthread");
    
    // We don't link against libhccl directly, as we use dlopen in bridge.cpp
    
    println!("cargo::rustc-cfg=cargo");
    println!("cargo::rustc-check-cfg=cfg(cargo)");
}
