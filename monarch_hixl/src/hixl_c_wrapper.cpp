#include "hixl_c_wrapper.h"
#include "hixl/hixl.h"
#include <string>
#include <vector>
#include <iostream>

static thread_local std::string g_last_error;

const char* hixl_get_error_msg() {
    return g_last_error.c_str();
}

HixlHandle hixl_create() {
    try {
        return new hixl::Hixl();
    } catch (const std::exception& e) {
        g_last_error = e.what();
        return nullptr;
    }
}

void hixl_destroy(HixlHandle handle) {
    if (handle) {
        delete static_cast<hixl::Hixl*>(handle);
    }
}

HixlStatus hixl_initialize(HixlHandle handle, const char* local_engine) {
    if (!handle || !local_engine) return HIXL_STATUS_ERROR;
    auto hixl = static_cast<hixl::Hixl*>(handle);
    std::map<hixl::AscendString, hixl::AscendString> options;
    // Default options?
    auto status = hixl->Initialize(local_engine, options);
    if (status != hixl::SUCCESS) {
        g_last_error = "Initialize failed with status: " + std::to_string(status);
        return HIXL_STATUS_ERROR;
    }
    return HIXL_STATUS_SUCCESS;
}

void hixl_finalize(HixlHandle handle) {
    if (handle) {
        static_cast<hixl::Hixl*>(handle)->Finalize();
    }
}

HixlStatus hixl_register_mem(HixlHandle handle, uintptr_t addr, size_t len, HixlMemType type, HixlMemHandle* out_handle) {
    if (!handle || !out_handle) return HIXL_STATUS_ERROR;
    auto hixl = static_cast<hixl::Hixl*>(handle);
    
    hixl::MemDesc desc;
    desc.addr = addr;
    desc.len = len;
    
    hixl::MemType mtype = (type == HIXL_MEM_DEVICE) ? hixl::MEM_DEVICE : hixl::MEM_HOST;
    hixl::MemHandle mhandle;
    
    auto status = hixl->RegisterMem(desc, mtype, mhandle);
    if (status != hixl::SUCCESS) {
        g_last_error = "RegisterMem failed with status: " + std::to_string(status);
        return HIXL_STATUS_ERROR;
    }
    *out_handle = mhandle;
    return HIXL_STATUS_SUCCESS;
}

HixlStatus hixl_deregister_mem(HixlHandle handle, HixlMemHandle mem_handle) {
    if (!handle) return HIXL_STATUS_ERROR;
    auto hixl = static_cast<hixl::Hixl*>(handle);
    auto status = hixl->DeregisterMem(mem_handle);
    if (status != hixl::SUCCESS) {
         g_last_error = "DeregisterMem failed with status: " + std::to_string(status);
         return HIXL_STATUS_ERROR;
    }
    return HIXL_STATUS_SUCCESS;
}

HixlStatus hixl_connect(HixlHandle handle, const char* remote_engine, int32_t timeout_ms) {
    if (!handle || !remote_engine) return HIXL_STATUS_ERROR;
    auto hixl = static_cast<hixl::Hixl*>(handle);
    auto status = hixl->Connect(remote_engine, timeout_ms);
    if (status != hixl::SUCCESS && status != hixl::ALREADY_CONNECTED) {
        g_last_error = "Connect failed with status: " + std::to_string(status);
        return HIXL_STATUS_ERROR;
    }
    return HIXL_STATUS_SUCCESS;
}

HixlStatus hixl_disconnect(HixlHandle handle, const char* remote_engine, int32_t timeout_ms) {
    if (!handle || !remote_engine) return HIXL_STATUS_ERROR;
    auto hixl = static_cast<hixl::Hixl*>(handle);
    auto status = hixl->Disconnect(remote_engine, timeout_ms);
    if (status != hixl::SUCCESS && status != hixl::NOT_CONNECTED) {
        g_last_error = "Disconnect failed with status: " + std::to_string(status);
        return HIXL_STATUS_ERROR;
    }
    return HIXL_STATUS_SUCCESS;
}

HixlStatus hixl_transfer_async(HixlHandle handle, 
                              const char* remote_engine, 
                              HixlOpType op, 
                              const HixlOpDesc* descriptors, 
                              size_t desc_count, 
                              HixlRequestHandle* out_req) {
    if (!handle || !remote_engine || !descriptors || !out_req) return HIXL_STATUS_ERROR;
    auto hixl = static_cast<hixl::Hixl*>(handle);
    
    std::vector<hixl::TransferOpDesc> ops;
    ops.reserve(desc_count);
    for (size_t i = 0; i < desc_count; ++i) {
        hixl::TransferOpDesc d;
        d.local_addr = descriptors[i].local_addr;
        d.remote_addr = descriptors[i].remote_addr;
        d.len = descriptors[i].len;
        ops.push_back(d);
    }
    
    hixl::TransferOp top = (op == HIXL_OP_READ) ? hixl::READ : hixl::WRITE;
    hixl::TransferArgs args;
    hixl::TransferReq req;
    
    auto status = hixl->TransferAsync(remote_engine, top, ops, args, req);
    if (status != hixl::SUCCESS) {
        g_last_error = "TransferAsync failed with status: " + std::to_string(status);
        return HIXL_STATUS_ERROR;
    }
    
    *out_req = req;
    return HIXL_STATUS_SUCCESS;
}

int hixl_check_transfer_status(HixlHandle handle, HixlRequestHandle req) {
    if (!handle) return -1;
    auto hixl = static_cast<hixl::Hixl*>(handle);
    
    hixl::TransferStatus status;
    auto res = hixl->GetTransferStatus(req, status);
    if (res != hixl::SUCCESS) {
        g_last_error = "GetTransferStatus failed with status: " + std::to_string(res);
        return -1;
    }
    
    if (status == hixl::TransferStatus::COMPLETED) return 0;
    if (status == hixl::TransferStatus::WAITING) return 1;
    if (status == hixl::TransferStatus::FAILED) return -2;
    if (status == hixl::TransferStatus::TIMEOUT) return -3;
    
    return -1;
}

