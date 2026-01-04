use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hyperactor::Actor;
use hyperactor::ActorId;
use hyperactor::ActorRef;
use hyperactor::Context;
use hyperactor::HandleClient;
use hyperactor::Handler;
use hyperactor::Instance;
use hyperactor::Named;
use hyperactor::OncePortRef;
use hyperactor::RefClient;
use hyperactor::RemoteSpawn;
use serde::Deserialize;
use serde::Serialize;
use anyhow::Result;

use crate::rdma_components::*;
use crate::bindings::*;

// Messages (Same as monarch_rdma)
#[derive(Handler, HandleClient, RefClient, Debug, Serialize, Deserialize, Named)]
pub enum RdmaManagerMessage {
    RequestBuffer {
        addr: usize,
        size: usize,
        #[reply]
        reply: OncePortRef<RdmaBuffer>,
    },
    ReleaseBuffer {
        buffer: RdmaBuffer,
    },
    RequestQueuePair {
        other: ActorRef<RdmaManagerActor>,
        self_device: String,
        other_device: String,
        #[reply]
        reply: OncePortRef<RdmaQueuePair>,
    },
    // Compat messages
    Connect {
        other: ActorRef<RdmaManagerActor>,
        self_device: String,
        other_device: String,
        endpoint: RdmaQpInfo,
    },
    InitializeQP {
        other: ActorRef<RdmaManagerActor>,
        self_device: String,
        other_device: String,
        #[reply]
        reply: OncePortRef<bool>,
    },
    ConnectionInfo {
        other: ActorRef<RdmaManagerActor>,
        self_device: String,
        other_device: String,
        #[reply]
        reply: OncePortRef<RdmaQpInfo>,
    },
    ReleaseQueuePair {
        other: ActorRef<RdmaManagerActor>,
        self_device: String,
        other_device: String,
        qp: RdmaQueuePair,
    },
    GetQpState {
        other: ActorRef<RdmaManagerActor>,
        self_device: String,
        other_device: String,
        #[reply]
        reply: OncePortRef<u32>,
    },
}

#[derive(Debug)]
#[hyperactor::export(
    spawn = true,
    handlers = [
        RdmaManagerMessage,
    ],
)]
pub struct RdmaManagerActor {
    hixl_handle: usize, // HixlHandle as usize to be Send
    mr_map: HashMap<usize, usize>, // mr_id -> HixlMemHandle (as usize)
    next_mr_id: usize,
    local_engine: String,
    // We might need to track connections
    connected_peers: HashMap<String, bool>, // remote_engine -> connected
}

unsafe impl Send for RdmaManagerActor {}

impl Drop for RdmaManagerActor {
    fn drop(&mut self) {
        if self.hixl_handle != 0 {
             unsafe {
                 hixl_finalize(self.hixl_handle as HixlHandle);
                 hixl_destroy(self.hixl_handle as HixlHandle);
             }
        }
    }
}

#[async_trait]
impl RemoteSpawn for RdmaManagerActor {
    type Params = Option<IbverbsConfig>; // Reuse dummy config or string

    async fn new(params: Self::Params) -> Result<Self, anyhow::Error> {
        let handle = unsafe { hixl_create() };
        if handle.is_null() {
            return Err(anyhow::anyhow!("Failed to create Hixl instance: {}", 
                unsafe { std::ffi::CStr::from_ptr(hixl_get_error_msg()).to_string_lossy() }));
        }

        // Initialize Hixl
        // We need local engine name (IP?).
        // For now, use "127.0.0.1" or env var?
        // Ideally params should contain it.
        // Assuming params.device contains it if provided.
        let local_engine = if let Some(cfg) = params {
            cfg.device
        } else {
            std::env::var("MONARCH_HIXL_ENGINE").unwrap_or_else(|_| "127.0.0.1".to_string())
        };

        unsafe {
            let status = hixl_initialize(handle, local_engine.as_ptr() as *const i8);
            if status != HixlStatus::HIXL_STATUS_SUCCESS {
                 let msg = std::ffi::CStr::from_ptr(hixl_get_error_msg()).to_string_lossy();
                 hixl_destroy(handle);
                 return Err(anyhow::anyhow!("Hixl initialize failed: {}", msg));
            }
        }

        Ok(Self {
            hixl_handle: handle as usize,
            mr_map: HashMap::new(),
            next_mr_id: 1,
            local_engine,
            connected_peers: HashMap::new(),
        })
    }
}

#[async_trait]
impl Actor for RdmaManagerActor {
    async fn init(&mut self, _this: &Instance<Self>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

#[async_trait]
#[hyperactor::forward(RdmaManagerMessage)]
impl RdmaManagerMessageHandler for RdmaManagerActor {
    async fn request_buffer(
        &mut self,
        cx: &Context<Self>,
        addr: usize,
        size: usize,
    ) -> Result<RdmaBuffer, anyhow::Error> {
        let handle = self.hixl_handle as HixlHandle;
        let mut mem_handle: HixlMemHandle = std::ptr::null_mut();
        
        // Assume host memory for now. 
        // TODO: Detect if device memory (CUDA)
        let mem_type = HixlMemType::HIXL_MEM_HOST; 
        
        unsafe {
            let status = hixl_register_mem(handle, addr as uintptr_t, size, mem_type, &mut mem_handle);
            if status != HixlStatus::HIXL_STATUS_SUCCESS {
                let msg = std::ffi::CStr::from_ptr(hixl_get_error_msg()).to_string_lossy();
                return Err(anyhow::anyhow!("RegisterMem failed: {}", msg));
            }
        }
        
        let mr_id = self.next_mr_id;
        self.next_mr_id += 1;
        self.mr_map.insert(mr_id, mem_handle as usize);

        Ok(RdmaBuffer {
            owner: cx.bind().clone(),
            mr_id,
            addr,
            size,
            lkey: 0,
            rkey: 0,
            device_name: self.local_engine.clone(),
        })
    }

    async fn release_buffer(
        &mut self,
        _cx: &Context<Self>,
        buffer: RdmaBuffer,
    ) -> Result<(), anyhow::Error> {
        if let Some(mem_handle_val) = self.mr_map.remove(&buffer.mr_id) {
            let handle = self.hixl_handle as HixlHandle;
            let mem_handle = mem_handle_val as HixlMemHandle;
            unsafe {
                hixl_deregister_mem(handle, mem_handle);
            }
        }
        Ok(())
    }

    async fn request_queue_pair(
        &mut self,
        _cx: &Context<Self>,
        _other: ActorRef<RdmaManagerActor>,
        _self_device: String,
        other_device: String,
    ) -> Result<RdmaQueuePair, anyhow::Error> {
        // Connect if not already connected
        // For Hixl, Connect is per remote engine.
        // other_device should be the remote engine name.
        
        if !self.connected_peers.contains_key(&other_device) {
             let handle = self.hixl_handle as HixlHandle;
             unsafe {
                 let status = hixl_connect(handle, other_device.as_ptr() as *const i8, 2000); // 2s timeout
                 if status != HixlStatus::HIXL_STATUS_SUCCESS {
                      // Maybe already connected?
                      // If Hixl handles duplicates gracefully, we are fine.
                      // Wrapper returns success if ALREADY_CONNECTED.
                      let msg = std::ffi::CStr::from_ptr(hixl_get_error_msg()).to_string_lossy();
                      return Err(anyhow::anyhow!("Connect failed to {}: {}", other_device, msg));
                 }
             }
             self.connected_peers.insert(other_device.clone(), true);
        }

        Ok(RdmaQueuePair {
            hixl_handle: self.hixl_handle,
            remote_engine: other_device,
        })
    }
    
    // Compat methods
    async fn connect(
        &mut self,
        _cx: &Context<Self>,
        _other: ActorRef<RdmaManagerActor>,
        _self_device: String,
        _other_device: String,
        _endpoint: RdmaQpInfo,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn initialize_qp(
        &mut self,
        _cx: &Context<Self>,
        _other: ActorRef<RdmaManagerActor>,
        _self_device: String,
        _other_device: String,
    ) -> Result<bool, anyhow::Error> {
        Ok(true)
    }

    async fn connection_info(
        &mut self,
        _cx: &Context<Self>,
        _other: ActorRef<RdmaManagerActor>,
        _self_device: String,
        _other_device: String,
    ) -> Result<RdmaQpInfo, anyhow::Error> {
        Ok(RdmaQpInfo {
             qp_num: 0,
             lid: 0,
             gid: None,
             psn: 0,
        })
    }

    async fn release_queue_pair(
        &mut self,
        _cx: &Context<Self>,
        _other: ActorRef<RdmaManagerActor>,
        _self_device: String,
        _other_device: String,
        _qp: RdmaQueuePair,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn get_qp_state(
        &mut self,
        _cx: &Context<Self>,
        _other: ActorRef<RdmaManagerActor>,
        _self_device: String,
        _other_device: String,
    ) -> Result<u32, anyhow::Error> {
        Ok(2) // RTS
    }
}

