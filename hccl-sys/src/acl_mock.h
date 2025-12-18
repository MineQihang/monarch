#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void* aclrtStream;
typedef void* aclrtDrvMemHandle;

// Minimal ACL types needed for bindings if headers are missing
typedef int32_t aclError;
const int32_t ACL_SUCCESS = 0;

#ifdef __cplusplus
}
#endif
