/*
 * Copyright (c) Huawei Technologies Co., Ltd.
 * All rights reserved.
 *
 * This source code is licensed under the BSD-style license found in the
 * LICENSE file in the root directory of this source tree.
 */

#pragma once

#include <limits.h>
#include <stddef.h>
#include <stdint.h>

// Mock or include ACL
#if __has_include(<acl/acl.h>)
#  include <acl/acl.h>
#else
#  include "acl_mock.h"
#endif

#include "../cann-hccl/inc/hccl/hccl.h"
#include "../cann-hccl/inc/hccl/hccl_types.h"

#ifdef __cplusplus
extern "C" {
#endif

// We can add any extra helper functions or types here if needed
// For now, we rely on hccl.h and hccl_types.h

// Helper ACL functions bound via dlopen
aclError aclrtSetDevice(int32_t deviceId);
aclError aclrtStreamSynchronize(aclrtStream stream);

#ifdef __cplusplus
} // extern "C"
#endif
