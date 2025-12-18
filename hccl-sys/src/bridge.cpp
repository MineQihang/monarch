/*
 * Copyright (c) Huawei Technologies Co., Ltd.
 * All rights reserved.
 *
 * This source code is licensed under the BSD-style license found in the
 * LICENSE file in the root directory of this source tree.
 */

#include "bridge.h"
#include <dlfcn.h>
#include <iostream>
#include <vector>

namespace hccl_sys {

struct HcclAPI {
    // Initialization
    HcclResult (*HcclCommInitClusterInfo_)(const char*, uint32_t, HcclComm*);
    HcclResult (*HcclCommInitAll_)(uint32_t, int32_t*, HcclComm*);
    HcclResult (*HcclGetRootInfo_)(HcclRootInfo*);
    HcclResult (*HcclCommInitRootInfo_)(uint32_t, const HcclRootInfo*, uint32_t, HcclComm*);
    
    // Comm management
    HcclResult (*HcclCommDestroy_)(HcclComm);
    HcclResult (*HcclGetRankSize_)(HcclComm, uint32_t*);
    HcclResult (*HcclGetRankId_)(HcclComm, uint32_t*);
    
    // Collectives
    HcclResult (*HcclAllReduce_)(void*, void*, uint64_t, HcclDataType, HcclReduceOp, HcclComm, aclrtStream);
    HcclResult (*HcclBroadcast_)(void*, uint64_t, HcclDataType, uint32_t, HcclComm, aclrtStream);
    HcclResult (*HcclAllGather_)(void*, void*, uint64_t, HcclDataType, HcclComm, aclrtStream);
    HcclResult (*HcclReduceScatter_)(void*, void*, uint64_t, HcclDataType, HcclReduceOp, HcclComm, aclrtStream);
    HcclResult (*HcclReduce_)(void*, void*, uint64_t, HcclDataType, HcclReduceOp, uint32_t, HcclComm, aclrtStream);
    HcclResult (*HcclAlltoAll_)(const void*, uint64_t, HcclDataType, const void*, uint64_t, HcclDataType, HcclComm, aclrtStream);
    HcclResult (*HcclAlltoAllV_)(const void*, const void*, const void*, HcclDataType, const void*, const void*, const void*, HcclDataType, HcclComm, aclrtStream);
    HcclResult (*HcclSend_)(void*, uint64_t, HcclDataType, uint32_t, HcclComm, aclrtStream);
    HcclResult (*HcclRecv_)(void*, uint64_t, HcclDataType, uint32_t, HcclComm, aclrtStream);
    HcclResult (*HcclBarrier_)(HcclComm, aclrtStream);

    // Error
    HcclResult (*HcclGetCommAsyncError_)(HcclComm, HcclResult*);
    const char* (*HcclGetErrorString_)(HcclResult);

    HcclResult init_result_;

    static HcclAPI* get();
};

namespace {

HcclAPI create_hccl_api() {
    HcclAPI r{};
    r.init_result_ = HCCL_SUCCESS;

    void* handle = dlopen("libhccl.so", RTLD_LAZY | RTLD_NOLOAD);
    if (!handle) {
        handle = dlopen("libhccl.so", RTLD_LAZY);
    }

    if (!handle) {
        // Try finding it in standard paths if not found
        // This might be redundant if LD_LIBRARY_PATH is set
        handle = dlopen("/usr/local/Ascend/ascend-toolkit/latest/lib64/libhccl.so", RTLD_LAZY);
    }

    if (!handle) {
        std::cerr << "[HCCL-SYS] Warning: Can't open libhccl.so: " << dlerror() << std::endl;
        r.init_result_ = HCCL_E_INTERNAL; // Use closest error code
        return r;
    }

#define LOOKUP_HCCL_ENTRY(name) \
    r.name##_ = reinterpret_cast<decltype(r.name##_)>(dlsym(handle, #name)); \
    if (!r.name##_) { \
        std::cerr << "[HCCL-SYS] Warning: Can't find " << #name << ": " << dlerror() << std::endl; \
        r.init_result_ = HCCL_E_INTERNAL; \
        return r; \
    }

    LOOKUP_HCCL_ENTRY(HcclCommInitClusterInfo)
    LOOKUP_HCCL_ENTRY(HcclCommInitAll)
    LOOKUP_HCCL_ENTRY(HcclGetRootInfo)
    LOOKUP_HCCL_ENTRY(HcclCommInitRootInfo)
    LOOKUP_HCCL_ENTRY(HcclCommDestroy)
    LOOKUP_HCCL_ENTRY(HcclGetRankSize)
    LOOKUP_HCCL_ENTRY(HcclGetRankId)
    LOOKUP_HCCL_ENTRY(HcclAllReduce)
    LOOKUP_HCCL_ENTRY(HcclBroadcast)
    LOOKUP_HCCL_ENTRY(HcclAllGather)
    LOOKUP_HCCL_ENTRY(HcclReduceScatter)
    LOOKUP_HCCL_ENTRY(HcclReduce)
    LOOKUP_HCCL_ENTRY(HcclAlltoAll)
    LOOKUP_HCCL_ENTRY(HcclAlltoAllV)
    LOOKUP_HCCL_ENTRY(HcclSend)
    LOOKUP_HCCL_ENTRY(HcclRecv)
    LOOKUP_HCCL_ENTRY(HcclBarrier)
    LOOKUP_HCCL_ENTRY(HcclGetCommAsyncError)
    LOOKUP_HCCL_ENTRY(HcclGetErrorString)

#undef LOOKUP_HCCL_ENTRY

    return r;
}

} // namespace

HcclAPI* HcclAPI::get() {
    static HcclAPI singleton = create_hccl_api();
    return &singleton;
}

} // namespace hccl_sys

namespace acl_sys {

struct AclAPI {
    aclError (*aclrtSetDevice_)(int32_t);
    aclError (*aclrtStreamSynchronize_)(aclrtStream);

    int init_result_;

    static AclAPI* get();
};

namespace {

AclAPI create_acl_api() {
    AclAPI r{};
    r.init_result_ = 0; // ACL_SUCCESS

    // Try libascendcl.so or libacl.so depending on version, usually libascendcl.so
    void* handle = dlopen("libascendcl.so", RTLD_LAZY | RTLD_NOLOAD);
    if (!handle) {
        handle = dlopen("libascendcl.so", RTLD_LAZY);
    }
    
    // Fallback or additional paths if needed
    if (!handle) {
         handle = dlopen("/usr/local/Ascend/ascend-toolkit/latest/lib64/libascendcl.so", RTLD_LAZY);
    }

    if (!handle) {
        std::cerr << "[HCCL-SYS] Warning: Can't open libascendcl.so: " << dlerror() << std::endl;
        r.init_result_ = -1; 
        return r;
    }

#define LOOKUP_ACL_ENTRY(name) \
    r.name##_ = reinterpret_cast<decltype(r.name##_)>(dlsym(handle, #name)); \
    if (!r.name##_) { \
        std::cerr << "[HCCL-SYS] Warning: Can't find " << #name << ": " << dlerror() << std::endl; \
        r.init_result_ = -1; \
        return r; \
    }

    LOOKUP_ACL_ENTRY(aclrtSetDevice)
    LOOKUP_ACL_ENTRY(aclrtStreamSynchronize)

#undef LOOKUP_ACL_ENTRY

    return r;
}

} // namespace

AclAPI* AclAPI::get() {
    static AclAPI singleton = create_acl_api();
    return &singleton;
}

} // namespace acl_sys


#define GET_HCCL_API(api_ptr) \
    hccl_sys::HcclAPI* api_ptr = hccl_sys::HcclAPI::get(); \
    if (api_ptr->init_result_ != HCCL_SUCCESS) { \
        return api_ptr->init_result_; \
    }

#define GET_ACL_API(api_ptr) \
    acl_sys::AclAPI* api_ptr = acl_sys::AclAPI::get(); \
    if (api_ptr->init_result_ != 0) { \
        return api_ptr->init_result_; \
    }

extern "C" {

HcclResult HcclCommInitClusterInfo(const char *clusterInfo, uint32_t rank, HcclComm *comm) {
    GET_HCCL_API(api);
    return api->HcclCommInitClusterInfo_(clusterInfo, rank, comm);
}

HcclResult HcclCommInitAll(uint32_t ndev, int32_t* devices, HcclComm* comms) {
    GET_HCCL_API(api);
    return api->HcclCommInitAll_(ndev, devices, comms);
}

HcclResult HcclGetRootInfo(HcclRootInfo *rootInfo) {
    GET_HCCL_API(api);
    return api->HcclGetRootInfo_(rootInfo);
}

HcclResult HcclCommInitRootInfo(uint32_t nRanks, const HcclRootInfo *rootInfo, uint32_t rank, HcclComm *comm) {
    GET_HCCL_API(api);
    return api->HcclCommInitRootInfo_(nRanks, rootInfo, rank, comm);
}

HcclResult HcclCommDestroy(HcclComm comm) {
    GET_HCCL_API(api);
    return api->HcclCommDestroy_(comm);
}

HcclResult HcclGetRankSize(HcclComm comm, uint32_t *rankSize) {
    GET_HCCL_API(api);
    return api->HcclGetRankSize_(comm, rankSize);
}

HcclResult HcclGetRankId(HcclComm comm, uint32_t *rank) {
    GET_HCCL_API(api);
    return api->HcclGetRankId_(comm, rank);
}

HcclResult HcclAllReduce(void *sendBuf, void *recvBuf, uint64_t count, HcclDataType dataType,
    HcclReduceOp op, HcclComm comm, aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclAllReduce_(sendBuf, recvBuf, count, dataType, op, comm, stream);
}

HcclResult HcclBroadcast(void *buf, uint64_t count, HcclDataType dataType, uint32_t root, HcclComm comm,
    aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclBroadcast_(buf, count, dataType, root, comm, stream);
}

HcclResult HcclAllGather(void *sendBuf, void *recvBuf, uint64_t sendCount, HcclDataType dataType,
    HcclComm comm, aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclAllGather_(sendBuf, recvBuf, sendCount, dataType, comm, stream);
}

HcclResult HcclReduceScatter(void *sendBuf, void *recvBuf, uint64_t recvCount, HcclDataType dataType,
    HcclReduceOp op, HcclComm comm, aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclReduceScatter_(sendBuf, recvBuf, recvCount, dataType, op, comm, stream);
}

HcclResult HcclReduce(void *sendBuf, void *recvBuf, uint64_t count, HcclDataType dataType,
    HcclReduceOp op, uint32_t root, HcclComm comm, aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclReduce_(sendBuf, recvBuf, count, dataType, op, root, comm, stream);
}

HcclResult HcclAlltoAll(const void *sendBuf, uint64_t sendCount, HcclDataType sendType,
                               const void *recvBuf, uint64_t recvCount, HcclDataType recvType,
                               HcclComm comm, aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclAlltoAll_(sendBuf, sendCount, sendType, recvBuf, recvCount, recvType, comm, stream);
}

HcclResult HcclAlltoAllV(const void *sendBuf, const void *sendCounts, const void *sdispls, HcclDataType sendType,
                         const void *recvBuf, const void *recvCounts, const void *rdispls, HcclDataType recvType,
                         HcclComm comm, aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclAlltoAllV_(sendBuf, sendCounts, sdispls, sendType, recvBuf, recvCounts, rdispls, recvType, comm, stream);
}

HcclResult HcclSend(void* sendBuf, uint64_t count, HcclDataType dataType, uint32_t destRank,
                           HcclComm comm, aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclSend_(sendBuf, count, dataType, destRank, comm, stream);
}

HcclResult HcclRecv(void* recvBuf, uint64_t count, HcclDataType dataType, uint32_t srcRank,
                           HcclComm comm, aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclRecv_(recvBuf, count, dataType, srcRank, comm, stream);
}

HcclResult HcclBarrier(HcclComm comm, aclrtStream stream) {
    GET_HCCL_API(api);
    return api->HcclBarrier_(comm, stream);
}

HcclResult HcclGetCommAsyncError(HcclComm comm, HcclResult *asyncError) {
    GET_HCCL_API(api);
    return api->HcclGetCommAsyncError_(comm, asyncError);
}

const char *HcclGetErrorString(HcclResult code) {
    hccl_sys::HcclAPI* api = hccl_sys::HcclAPI::get();
    if (api->init_result_ != HCCL_SUCCESS) {
        return "HCCL library not initialized";
    }
    return api->HcclGetErrorString_(code);
}

aclError aclrtSetDevice(int32_t deviceId) {
    GET_ACL_API(api);
    return api->aclrtSetDevice_(deviceId);
}

aclError aclrtStreamSynchronize(aclrtStream stream) {
    GET_ACL_API(api);
    return api->aclrtStreamSynchronize_(stream);
}

} // extern "C"
