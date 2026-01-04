mod bindings;
pub mod rdma_components;
pub mod rdma_manager_actor;

pub use rdma_components::*;
pub use rdma_manager_actor::*;

pub fn rdma_supported() -> bool {
    // TODO: stricter check?
    true
}

// Dummy/Compat
pub fn print_device_info_if_debug_enabled(_context: *mut std::ffi::c_void) {}
pub fn print_device_info(_context: *mut std::ffi::c_void) {}

