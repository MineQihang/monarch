use hyperactor::ActorRef;
use hyperactor::Named;
use hyperactor::context;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;
use hyperactor::clock::RealClock;
use hyperactor::clock::Clock;

use crate::RdmaManagerActor;
use crate::bindings::*;

#[derive(Debug, Serialize, Deserialize, Named, Clone)]
pub struct RdmaBuffer {
    pub owner: ActorRef<RdmaManagerActor>,
    pub mr_id: usize, // Mapped to HixlMemHandle (casted to usize)
    pub lkey: u32,    // Dummy for Hixl
    pub rkey: u32,    // Dummy for Hixl
    pub addr: usize,
    pub size: usize,
    pub device_name: String, // Hixl engine name?
}

impl RdmaBuffer {
    pub async fn read_into(
        &self,
        client: &impl context::Actor,
        remote: RdmaBuffer,
        timeout: u64,
    ) -> Result<bool, anyhow::Error> {
        // Hixl read: read from remote to local (self)
        // Operation: READ.
        // Local: self.addr
        // Remote: remote.addr
        
        // We need a QueuePair-like object to perform the operation.
        // In monarch_rdma, we request a QP from the owner.
        // Here we do the same to get a handle to Hixl connection.
        
        let remote_owner = remote.owner.clone();
        let local_device = self.device_name.clone();
        let remote_device = remote.device_name.clone();

        let mut qp = self.owner
            .request_queue_pair(
                client,
                remote_owner.clone(),
                local_device.clone(),
                remote_device.clone(),
            )
            .await?;

        let wr_ids = qp.get(self.clone(), remote)?;
        // Wait for completion
        let result = self.wait_for_completion(&mut qp, &wr_ids, timeout).await;

        self.owner
            .release_queue_pair(client, remote_owner, local_device, remote_device, qp)
            .await?;

        result
    }

    pub async fn write_from(
        &self,
        client: &impl context::Actor,
        remote: RdmaBuffer,
        timeout: u64,
    ) -> Result<bool, anyhow::Error> {
        // Hixl write: write from remote (source) to self (dest)?
        // Wait, write_from description: "writing from the remote buffer into local memory"?
        // No, verify monarch_rdma doc:
        // "Writes data from this remote RDMA buffer into a local buffer."
        // "This operation appears as "write_from" from the caller's perspective (writing from the remote buffer into local memory), but internally it's implemented as a "read_into" operation on the local buffer since the data flows from the remote buffer to the local one."
        // Confusing doc.
        // Let's look at `read_into` impl in monarch_rdma:
        // `qp.put(self.clone(), remote)?`
        // self is local buffer. remote is the "other" buffer.
        // put(local, remote) means write FROM local TO remote.
        // So `read_into` (caller perspective: read local data into remote buffer) -> RDMA Write (Put).
        
        // `write_from` impl in monarch_rdma:
        // `qp.get(self.clone(), remote)?`
        // get(local, remote) means read FROM remote TO local.
        // So `write_from` (caller perspective: write local buffer with data from remote) -> RDMA Read (Get).

        // Hixl terminology:
        // READ: Remote -> Local
        // WRITE: Local -> Remote

        // So `read_into`: Local -> Remote (Hixl WRITE)
        // `write_from`: Remote -> Local (Hixl READ)

        // The implementation here mirrors monarch_rdma structure.
        let remote_owner = remote.owner.clone();
        let local_device = self.device_name.clone();
        let remote_device = remote.device_name.clone();

        let mut qp = self.owner
            .request_queue_pair(
                client,
                remote_owner.clone(),
                local_device.clone(),
                remote_device.clone(),
            )
            .await?;

        let wr_ids = qp.put(self.clone(), remote)?;
        let result = self.wait_for_completion(&mut qp, &wr_ids, timeout).await;

        self.owner
            .release_queue_pair(client, remote_owner, local_device, remote_device, qp)
            .await?;

        result
    }

    async fn wait_for_completion(
        &self,
        qp: &mut RdmaQueuePair,
        wr_ids: &[u64],
        timeout: u64,
    ) -> Result<bool, anyhow::Error> {
        let timeout = Duration::from_secs(timeout);
        let start_time = std::time::Instant::now();
        
        // Polling logic
        let mut remaining = wr_ids.to_vec();
        
        while start_time.elapsed() < timeout {
             if remaining.is_empty() {
                 return Ok(true);
             }
             
             // Check status of each request
             let mut completed = Vec::new();
             for &wr_id in &remaining {
                 match qp.check_status(wr_id) {
                     Ok(true) => completed.push(wr_id),
                     Ok(false) => {}, // Still waiting
                     Err(e) => return Err(e),
                 }
             }
             
             for id in completed {
                 remaining.retain(|&x| x != id);
             }
             
             if remaining.is_empty() {
                 return Ok(true);
             }
             
             RealClock.sleep(Duration::from_millis(1)).await;
        }
        
        Err(anyhow::anyhow!("Timeout waiting for completion"))
    }

    pub async fn drop_buffer(&self, client: &impl context::Actor) -> Result<(), anyhow::Error> {
        self.owner.release_buffer(client, self.clone()).await?;
        Ok(())
    }
}

// Dummy for compatibility
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PollTarget {
    Send,
    Recv,
}

#[derive(Debug, Serialize, Deserialize, Named, Clone)]
pub struct RdmaQueuePair {
    // We hold the Hixl handle (as usize to be Send/Sync/Serialize) and remote engine name
    pub hixl_handle: usize, 
    pub remote_engine: String,
    // We don't need CQs etc.
}

impl RdmaQueuePair {
    pub fn get(&mut self, local: RdmaBuffer, remote: RdmaBuffer) -> Result<Vec<u64>, anyhow::Error> {
        // Hixl READ: Remote -> Local
        self.transfer(local, remote, HixlOpType::HIXL_OP_READ)
    }

    pub fn put(&mut self, local: RdmaBuffer, remote: RdmaBuffer) -> Result<Vec<u64>, anyhow::Error> {
        // Hixl WRITE: Local -> Remote
        self.transfer(local, remote, HixlOpType::HIXL_OP_WRITE)
    }
    
    fn transfer(&mut self, local: RdmaBuffer, remote: RdmaBuffer, op: HixlOpType) -> Result<Vec<u64>, anyhow::Error> {
        let handle = self.hixl_handle as HixlHandle;
        
        let desc = HixlOpDesc {
            local_addr: local.addr as uintptr_t,
            remote_addr: remote.addr as uintptr_t,
            len: local.size, // Assuming sizes match or using local size
        };
        
        let mut req: HixlRequestHandle = std::ptr::null_mut();
        // unsafe call
        unsafe {
            let status = hixl_transfer_async(
                handle,
                self.remote_engine.as_ptr() as *const i8,
                op,
                &desc,
                1,
                &mut req
            );
            
            if status != HixlStatus::HIXL_STATUS_SUCCESS {
                 let msg = std::ffi::CStr::from_ptr(hixl_get_error_msg()).to_string_lossy();
                 return Err(anyhow::anyhow!("Hixl transfer failed: {}", msg));
            }
        }
        
        Ok(vec![req as usize as u64])
    }
    
    pub fn check_status(&self, wr_id: u64) -> Result<bool, anyhow::Error> {
        let handle = self.hixl_handle as HixlHandle;
        let req = wr_id as usize as HixlRequestHandle;
        
        unsafe {
            let res = hixl_check_transfer_status(handle, req);
            if res == 0 { return Ok(true); } // Completed
            if res == 1 { return Ok(false); } // Waiting
            
            let msg = std::ffi::CStr::from_ptr(hixl_get_error_msg()).to_string_lossy();
            return Err(anyhow::anyhow!("Hixl transfer error code {}: {}", res, msg));
        }
    }

    // Stub for connect if needed, but connection is handled at actor level
    pub fn connect(&mut self, _info: &RdmaQpInfo) -> Result<(), anyhow::Error> {
        // Already connected by actor
        Ok(())
    }
    
    pub fn get_qp_info(&mut self) -> Result<RdmaQpInfo, anyhow::Error> {
        Ok(RdmaQpInfo {
            // Dummy info
            qp_num: 0,
            lid: 0,
            gid: None,
            psn: 0,
        })
    }
    
    pub fn state(&mut self) -> Result<u32, anyhow::Error> {
        Ok(2) // RTS
    }
}

// Dummy types to match monarch_rdma API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RdmaQpInfo {
    pub qp_num: u32,
    pub lid: u16,
    pub gid: Option<Vec<u8>>, // Simplified Gid
    pub psn: u32,
}

#[derive(Debug, Clone, Default)]
pub struct IbverbsConfig {
    pub device: String,
    // ... other fields if needed
}

