#ifndef HIXL_C_WRAPPER_H
#define HIXL_C_WRAPPER_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void* HixlHandle;
typedef void* HixlMemHandle;
typedef void* HixlRequestHandle;

typedef enum {
    HIXL_STATUS_SUCCESS = 0,
    HIXL_STATUS_ERROR = 1
} HixlStatus;

typedef enum {
    HIXL_MEM_DEVICE = 0,
    HIXL_MEM_HOST = 1
} HixlMemType;

typedef enum {
    HIXL_OP_READ = 0,
    HIXL_OP_WRITE = 1
} HixlOpType;

typedef struct {
    uintptr_t local_addr;
    uintptr_t remote_addr;
    size_t len;
} HixlOpDesc;

HixlHandle hixl_create();
void hixl_destroy(HixlHandle handle);

HixlStatus hixl_initialize(HixlHandle handle, const char* local_engine);
void hixl_finalize(HixlHandle handle);

HixlStatus hixl_register_mem(HixlHandle handle, uintptr_t addr, size_t len, HixlMemType type, HixlMemHandle* out_handle);
HixlStatus hixl_deregister_mem(HixlHandle handle, HixlMemHandle mem_handle);

HixlStatus hixl_connect(HixlHandle handle, const char* remote_engine, int32_t timeout_ms);
HixlStatus hixl_disconnect(HixlHandle handle, const char* remote_engine, int32_t timeout_ms);

// Simple async transfer - returns request handle
HixlStatus hixl_transfer_async(HixlHandle handle, 
                              const char* remote_engine, 
                              HixlOpType op, 
                              const HixlOpDesc* descriptors, 
                              size_t desc_count, 
                              HixlRequestHandle* out_req);

// Check status of request
// Returns 0 for success/complete, 1 for waiting/in-progress, <0 for error
int hixl_check_transfer_status(HixlHandle handle, HixlRequestHandle req);

// Get last error message (thread local)
const char* hixl_get_error_msg();

#ifdef __cplusplus
}
#endif

#endif // HIXL_C_WRAPPER_H

