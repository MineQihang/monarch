/*
 * Copyright (c) Huawei Technologies Co., Ltd.
 * All rights reserved.
 *
 * This source code is licensed under the BSD-style license found in the
 * LICENSE file in the root directory of this source tree.
 */

#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(non_snake_case)]
mod inner {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use inner::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_error_string() {
        // This tests if the bridge library is correctly linked and symbols are resolved.
        // HcclGetErrorString doesn't require initialization of the device.
        unsafe {
            let str_ptr = HcclGetErrorString(HcclResult(0)); // HCCL_SUCCESS
            if !str_ptr.is_null() {
                let c_str = CStr::from_ptr(str_ptr);
                println!("Error string for success: {:?}", c_str);
            }
        }
    }
}
