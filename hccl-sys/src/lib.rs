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
    use std::ptr;

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

    #[test]
    #[ignore] // Skip by default as it requires NPU environment and rank setup
    fn test_comm_init_and_p2p() {
        unsafe {
            // 1. Get rank info from environment (standard Ascend/HCCL env vars)
            // Note: In a real distributed run, these are set by mpirun or manually.
            let rank_id = std::env::var("RANK_ID").unwrap_or("0".to_string()).parse::<u32>().unwrap();
            let rank_size = std::env::var("RANK_SIZE").unwrap_or("1".to_string()).parse::<u32>().unwrap();
            
            // Basic check to ensure we have at least 2 ranks for P2P test if we want to really send/recv
            if rank_size < 2 {
                println!("Skipping P2P test part because RANK_SIZE < 2. Only testing init/destroy.");
            }

            // 2. Initialize HCCL
            // Assuming rank_table_file is set in RANK_TABLE_FILE env var if using HcclCommInitClusterInfo
            // For single-process multi-device, we would use HcclCommInitAll, but here we assume multi-process
            // which is more common for distributed training.
            
            // Let's try HcclGetRootInfo style if we were building a manager, but for simple test
            // we often rely on the cluster info file or just assume the env is ready.
            // Here we use a hypothetical simple init flow. 
            // NOTE: Requires 'rank_table_file.json' or similar if using HcclCommInitClusterInfo.
            // For this test, we'll assume we can use HcclCommInitClusterInfo if a file is present,
            // or just skip if not.
            
            let cluster_conf = std::env::var("RANK_TABLE_FILE").unwrap_or_default();
            if cluster_conf.is_empty() {
                println!("RANK_TABLE_FILE not set, skipping detailed comm test.");
                return;
            }
            
            let cluster_conf_c = std::ffi::CString::new(cluster_conf).unwrap();
            let mut comm: HcclComm = ptr::null_mut();
            
            let ret = HcclCommInitClusterInfo(cluster_conf_c.as_ptr(), rank_id, &mut comm);
            assert_eq!(ret.0, 0, "HcclCommInitClusterInfo failed");
            assert!(!comm.is_null(), "HcclComm is null");

            // 3. Verify Rank ID/Size
            let mut check_rank = 0;
            let mut check_size = 0;
            let ret = HcclGetRankId(comm, &mut check_rank);
            assert_eq!(ret.0, 0);
            assert_eq!(check_rank, rank_id);

            let ret = HcclGetRankSize(comm, &mut check_size);
            assert_eq!(ret.0, 0);
            assert_eq!(check_size, rank_size);

            // 4. Simple Send/Recv (Ping-Pong)
            // Rank 0 sends to Rank 1, Rank 1 receives from Rank 0
            if rank_size >= 2 {
                // Prepare device buffers (Mocking memory allocation here is tricky without ACL)
                // In a real test, we need aclrtMalloc. 
                // Since we don't have safe wrappers for ACL here yet, we assume the user
                // has set up the environment such that we can use a raw pointer or
                // we skip the actual execution if we can't malloc.
                
                // NOTE: We cannot easily alloc device memory without binding aclrtMalloc.
                // This test serves as a compilation and logic check for the bindings.
                // To actually run this, you'd need to link and use acl-sys or similar.
                println!("Skipping actual data transfer because device memory allocation (aclrtMalloc) is not bound in this crate yet.");
                
                /* 
                // Pseudo-code for what would follow:
                let count = 100;
                let size = count * std::mem::size_of::<f32>();
                let mut send_buff: *mut c_void = ptr::null_mut();
                let mut recv_buff: *mut c_void = ptr::null_mut();
                aclrtMalloc(&mut send_buff, size, ACL_MEM_MALLOC_HUGE_FIRST);
                aclrtMalloc(&mut recv_buff, size, ACL_MEM_MALLOC_HUGE_FIRST);
                
                let stream: aclrtStream = ptr::null_mut(); // Default stream or create one
                
                if rank_id == 0 {
                    HcclSend(send_buff, count as u64, HcclDataType::HCCL_DATA_TYPE_FP32, 1, comm, stream);
                } else if rank_id == 1 {
                    HcclRecv(recv_buff, count as u64, HcclDataType::HCCL_DATA_TYPE_FP32, 0, comm, stream);
                }
                
                aclrtSynchronizeStream(stream);
                aclrtFree(send_buff);
                aclrtFree(recv_buff);
                */
            }

            // 5. Destroy Comm
            let ret = HcclCommDestroy(comm);
            assert_eq!(ret.0, 0, "HcclCommDestroy failed");
        }
    }

    #[test]
    #[ignore]
    fn test_acl_bindings() {
        unsafe {
            // Check if ACL library symbols can be loaded
            // We use a dummy stream (null) for synchronization check just to see if the symbol resolves.
            // Actual execution would likely fail or do nothing if null is handled gracefully by ACL.
            // But main goal is to check linkage.
            
            // Note: aclrtSetDevice typically requires actual hardware.
            // If we are just compiling, this test ensures the binding exists.
            
            // Try setting device 0 (if env has devices)
            let ret = aclrtSetDevice(0);
            println!("aclrtSetDevice(0) returned: {}", ret);
            
            let stream: aclrtStream = ptr::null_mut();
            let ret = aclrtStreamSynchronize(stream);
            println!("aclrtStreamSynchronize(null) returned: {}", ret);
        }
    }
}
