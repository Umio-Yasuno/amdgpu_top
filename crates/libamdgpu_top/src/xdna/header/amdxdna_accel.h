/* SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note */
/*
 * Copyright (C) 2022-2025, Advanced Micro Devices, Inc.
 */

#ifndef AMDXDNA_ACCEL_H_
#define AMDXDNA_ACCEL_H_

#ifdef __KERNEL__
#include <drm/drm.h>
#else
#include <libdrm/drm.h>
#endif
#include <linux/const.h>
#include <linux/stddef.h>

#if defined(__cplusplus)
extern "C" {
#endif

#define AMDXDNA_DRIVER_MAJOR		1
#define AMDXDNA_DRIVER_MINOR		0

#define AMDXDNA_INVALID_ADDR		(~0UL)
#define AMDXDNA_INVALID_CTX_HANDLE	0
#define AMDXDNA_INVALID_BO_HANDLE	0
#define AMDXDNA_INVALID_FENCE_HANDLE	0

#define POWER_MODE_DEFAULT	0
#define POWER_MODE_LOW		1
#define POWER_MODE_MEDIUM	2
#define POWER_MODE_HIGH		3
#define POWER_MODE_TURBO	4

/*
 * The interface can grow/extend over time.
 * On each struct amdxdna_drm_*, to support potential extension, we defined it
 * like this.
 *
 * Example code:
 *
 * struct amdxdna_drm_example_data {
 *	.ext = (uintptr_t)&example_data_ext;
 *	...
 * };
 *
 * We don't have extension now. The extension struct will define in the future.
 */

#define	DRM_AMDXDNA_CREATE_CTX		0
#define	DRM_AMDXDNA_DESTROY_CTX		1
#define	DRM_AMDXDNA_CONFIG_CTX		2
#define	DRM_AMDXDNA_CREATE_BO		3
#define	DRM_AMDXDNA_GET_BO_INFO		4
#define	DRM_AMDXDNA_SYNC_BO		5
#define	DRM_AMDXDNA_EXEC_CMD		6
#define	DRM_AMDXDNA_GET_INFO		7
#define	DRM_AMDXDNA_SET_STATE		8
#define	DRM_AMDXDNA_WAIT_CMD		9

#define	AMDXDNA_DEV_TYPE_UNKNOWN	-1
#define	AMDXDNA_DEV_TYPE_KMQ		0
#define	AMDXDNA_DEV_TYPE_UMQ		1

/*
 * Define priority in application's QoS.
 * AMDXDNA_QOS_DEFAULT_PRIORITY: Driver decide priority for client.
 * AMDXDNA_QOS_REALTIME_PRIORITY: Real time clients.
 * AMDXDNA_QOS_HIGH_PRIORITY: Best effort foreground clients.
 * AMDXDNA_QOS_NORMAL_PRIORITY: Best effort or background clients.
 * AMDXDNA_QOS_LOW_PRIORITY: Clients that can wait indefinite amount of time for
 *                           completion.
 */
#define	AMDXDNA_QOS_DEFAULT_PRIORITY	0
#define	AMDXDNA_QOS_REALTIME_PRIORITY	1
#define	AMDXDNA_QOS_HIGH_PRIORITY	2
#define	AMDXDNA_QOS_NORMAL_PRIORITY	3
#define	AMDXDNA_QOS_LOW_PRIORITY	4
/* The maximum number of priority */
#define	AMDXDNA_NUM_PRIORITY		4

/**
 * struct qos_info - QoS information for driver.
 * @gops: Giga operations per second.
 * @fps: Frames per second.
 * @dma_bandwidth: DMA bandwidtha.
 * @latency: Frame response latency.
 * @frame_exec_time: Frame execution time.
 * @priority: Request priority.
 *
 * User program can provide QoS hints to driver.
 */
struct amdxdna_qos_info {
	__u32 gops;
	__u32 fps;
	__u32 dma_bandwidth;
	__u32 latency;
	__u32 frame_exec_time;
	__u32 priority;
};

/**
 * struct amdxdna_drm_create_ctx - Create context.
 * @ext: MBZ.
 * @ext_flags: MBZ.
 * @qos_p: Address of QoS info.
 * @umq_bo: BO handle for user mode queue(UMQ).
 * @log_buf_bo: BO handle for log buffer.
 * @max_opc: Maximum operations per cycle.
 * @num_tiles: Number of AIE tiles.
 * @mem_size: Size of AIE tile memory.
 * @umq_doorbell: Returned offset of doorbell associated with UMQ.
 * @handle: Returned context handle.
 * @syncobj_handle: The drm timeline syncobj handle for command completion notification.
 */
struct amdxdna_drm_create_ctx {
	__u64 ext;
	__u64 ext_flags;
	__u64 qos_p;
	__u32 umq_bo;
	__u32 log_buf_bo;
	__u32 max_opc;
	__u32 num_tiles;
	__u32 mem_size;
	__u32 umq_doorbell;
	__u32 handle;
	__u32 syncobj_handle;
};

/**
 * struct amdxdna_drm_destroy_ctx - Destroy context.
 * @handle: Context handle.
 * @pad: Structure padding.
 */
struct amdxdna_drm_destroy_ctx {
	__u32 handle;
	__u32 pad;
};

/**
 * struct amdxdna_cu_config - configuration for one CU
 * @cu_bo: CU configuration buffer bo handle.
 * @cu_func: Function of a CU.
 * @pad: Structure padding.
 */
struct amdxdna_cu_config {
	__u32 cu_bo;
	__u8  cu_func;
	__u8  pad[3];
};

/**
 * struct amdxdna_ctx_param_config_cu - configuration for CUs in context
 * @num_cus: Number of CUs to configure.
 * @pad: Structure padding.
 * @cu_configs: Array of CU configurations of struct amdxdna_cu_config.
 */
struct amdxdna_ctx_param_config_cu {
	__u16 num_cus;
	__u16 pad[3];
	struct amdxdna_cu_config cu_configs[];
};

/**
 * struct amdxdna_drm_config_ctx - Configure context.
 * @handle: Context handle.
 * @param_type: Specifies the structure passed in via param_val.
 * @param_val: A structure specified by the param_type struct member.
 * @param_val_size: Size of the parameter buffer pointed to by the param_val.
 *		    If param_val is not a pointer, driver can ignore this.
 * @pad: Structure padding.
 *
 * Note: if the param_val is a pointer pointing to a buffer, the maximum size
 * of the buffer is 4KiB(PAGE_SIZE).
 */
struct amdxdna_drm_config_ctx {
	__u32 handle;
#define DRM_AMDXDNA_CTX_CONFIG_CU	0
#define	DRM_AMDXDNA_CTX_ASSIGN_DBG_BUF	1
#define	DRM_AMDXDNA_CTX_REMOVE_DBG_BUF	2
	__u32 param_type;
	__u64 param_val;
	__u32 param_val_size;
	__u32 pad;
};

/**
 * struct amdxdna_bo_va_entry - virtual address list entry
 *
 * @vaddr: Virtual address
 * @len: Length of memory segment
 */
struct amdxdna_bo_va_entry {
	__u64	vaddr;
	__u64	len;
};

/**
 * struct amdxdna_drm_create_bo - Create a buffer object.
 * @flags: Buffer flags. MBZ.
 * @vaddr: User VA of buffer if applied. MBZ.
 * @size: Size in bytes.
 * @type: Buffer type.
 * @handle: Returned DRM buffer object handle.
 */
struct amdxdna_drm_create_bo {
	__u64	flags;
	__u64	vaddr;
	__u64	size;
/*
 * AMDXDNA_BO_SHARE:	Regular BO shared between user and device
 * AMDXDNA_BO_DEV_HEAP: Shared host memory to device as heap memory
 * AMDXDNA_BO_DEV_BO:	Allocated from BO_DEV_HEAP
 * AMDXDNA_BO_CMD:	User and driver accessible bo
 * AMDXDNA_BO_DMA:	DRM GEM DMA bo
 */
#define	AMDXDNA_BO_INVALID	0
#define	AMDXDNA_BO_SHARE	1
#define	AMDXDNA_BO_DEV_HEAP	2
#define	AMDXDNA_BO_DEV		3
#define	AMDXDNA_BO_CMD		4
#define	AMDXDNA_BO_DMA		5
#define	AMDXDNA_BO_GUEST	6
	__u32	type;
	__u32	handle;
};

/**
 * struct amdxdna_drm_get_bo_info - Get buffer object information.
 * @ext: MBZ.
 * @ext_flags: MBZ.
 * @handle: DRM buffer object handle.
 * @pad: Structure padding.
 * @map_offset: Returned DRM fake offset for mmap().
 * @vaddr: Returned user VA of buffer. 0 in case user needs mmap().
 * @xdna_addr: Returned XDNA device virtual address.
 */
struct amdxdna_drm_get_bo_info {
	__u64 ext;
	__u64 ext_flags;
	__u32 handle;
	__u32 pad;
	__u64 map_offset;
	__u64 vaddr;
	__u64 xdna_addr;
};

/**
 * struct amdxdna_drm_sync_bo - Sync buffer object.
 * @handle: Buffer object handle.
 * @direction: Direction of sync, can be from device or to device.
 * @offset: Offset in the buffer to sync.
 * @size: Size in bytes.
 */
struct amdxdna_drm_sync_bo {
	__u32 handle;
#define SYNC_DIRECT_TO_DEVICE	0U
#define SYNC_DIRECT_FROM_DEVICE	1U
	__u32 direction;
	__u64 offset;
	__u64 size;
};

/**
 * struct amdxdna_drm_exec_cmd - Execute command.
 * @ext: MBZ.
 * @ext_flags: MBZ.
 * @ctx: Context handle.
 * @type: Command type.
 * @cmd_handles: Array of command handles or the command handle itself
 *               in case of just one.
 * @args: Array of arguments for all command handles.
 * @cmd_count: Number of command handles in the cmd_handles array.
 * @arg_count: Number of arguments in the args array.
 * @seq: Returned sequence number for this command.
 */
struct amdxdna_drm_exec_cmd {
	__u64 ext;
	__u64 ext_flags;
	__u32 ctx;
#define	AMDXDNA_CMD_SUBMIT_EXEC_BUF	0
#define	AMDXDNA_CMD_SUBMIT_DEPENDENCY	1
#define	AMDXDNA_CMD_SUBMIT_SIGNAL	2
	__u32 type;
	__u64 cmd_handles;
	__u64 args;
	__u32 cmd_count;
	__u32 arg_count;
	__u64 seq;
};

/**
 * struct amdxdna_drm_wait_cmd - Wait exectuion command.
 *
 * @ctx: Context handle.
 * @timeout: timeout in ms, 0 implies infinite wait.
 * @seq: sequence number of the command returned by execute command.
 *
 * Wait a command specified by seq to be completed.
 */
struct amdxdna_drm_wait_cmd {
	__u32 ctx;
	__u32 timeout;
	__u64 seq;
};

/**
 * struct amdxdna_drm_query_aie_status - Query the status of the AIE hardware
 * @buffer: The user space buffer that will return the AIE status.
 * @buffer_size: The size of the user space buffer.
 * @cols_filled: A bitmap of AIE columns whose data has been returned in the buffer.
 */
struct amdxdna_drm_query_aie_status {
	__u64 buffer; /* out */
	__u32 buffer_size; /* in */
	__u32 cols_filled; /* out */
};

/**
 * struct amdxdna_drm_query_aie_version - Query the version of the AIE hardware
 * @major: The major version number.
 * @minor: The minor version number.
 */
struct amdxdna_drm_query_aie_version {
	__u32 major; /* out */
	__u32 minor; /* out */
};

/**
 * struct amdxdna_drm_query_aie_tile_metadata - Query the metadata of AIE tile (core, mem, shim)
 * @row_count: The number of rows.
 * @row_start: The starting row number.
 * @dma_channel_count: The number of dma channels.
 * @lock_count: The number of locks.
 * @event_reg_count: The number of events.
 * @pad: Structure padding.
 */
struct amdxdna_drm_query_aie_tile_metadata {
	__u16 row_count;
	__u16 row_start;
	__u16 dma_channel_count;
	__u16 lock_count;
	__u16 event_reg_count;
	__u16 pad[3];
};

/**
 * struct amdxdna_drm_query_aie_metadata - Query the metadata of the AIE hardware
 * @col_size: The size of a column in bytes.
 * @cols: The total number of columns.
 * @rows: The total number of rows.
 * @version: The version of the AIE hardware.
 * @core: The metadata for all core tiles.
 * @mem: The metadata for all mem tiles.
 * @shim: The metadata for all shim tiles.
 */
struct amdxdna_drm_query_aie_metadata {
	__u32 col_size;
	__u16 cols;
	__u16 rows;
	struct amdxdna_drm_query_aie_version version;
	struct amdxdna_drm_query_aie_tile_metadata core;
	struct amdxdna_drm_query_aie_tile_metadata mem;
	struct amdxdna_drm_query_aie_tile_metadata shim;
};

/**
 * struct amdxdna_drm_query_clock - Metadata for a clock
 * @name: The clock name.
 * @freq_mhz: The clock frequency.
 * @pad: Structure padding.
 */
struct amdxdna_drm_query_clock {
	__u8 name[16];
	__u32 freq_mhz;
	__u32 pad;
};

/**
 * struct amdxdna_drm_query_clock_metadata - Query metadata for clocks
 * @mp_npu_clock: The metadata for MP-NPU clock.
 * @h_clock: The metadata for H clock.
 */
struct amdxdna_drm_query_clock_metadata {
	struct amdxdna_drm_query_clock mp_npu_clock;
	struct amdxdna_drm_query_clock h_clock;
};

/**
 * struct amdxdna_drm_query_sensor - The data for single sensor.
 * @label: The name for a sensor.
 * @input: The current value of the sensor.
 * @max: The maximum value possible for the sensor.
 * @average: The average value of the sensor.
 * @highest: The highest recorded sensor value for this driver load for the sensor.
 * @status: The sensor status.
 * @units: The sensor units.
 * @unitm: Translates value member variables into the correct unit via (pow(10, unitm) * value).
 * @type: The sensor type.
 * @pad: Structure padding.
 */
struct amdxdna_drm_query_sensor {
	__u8  label[64];
	__u32 input;
	__u32 max;
	__u32 average;
	__u32 highest;
	__u8  status[64];
	__u8  units[16];
	__s8  unitm;
#define AMDXDNA_SENSOR_TYPE_POWER 0
	__u8  type;
	__u8  pad[6];
};

/**
 * struct amdxdna_drm_query_ctx - The data for single context.
 * @context_id: The ID for this context.
 * @start_col: The starting column for the partition assigned to this context.
 * @num_col: The number of columns in the partition assigned to this context.
 * @nwctx_id: Hardware context ID.
 * @pid: The Process ID of the process that created this context.
 * @command_submissions: The number of commands submitted to this context.
 * @command_completions: The number of commands completed by this context.
 * @migrations: The number of times this context has been moved to a different partition.
 * @preemptions: The number of times this context has been preempted by another context in the
 *               same partition.
 * @errors: The errors for this context.
 * @priority: Context priority
 */
struct amdxdna_drm_query_ctx {
	__u32 context_id;
	__u32 start_col;
	__u32 num_col;
	__u32 hwctx_id;
	__s64 pid;
	__u64 command_submissions;
	__u64 command_completions;
	__u64 migrations;
	__u64 preemptions;
	__u64 errors;
	__u64 priority;
};

/**
 * struct amdxdna_drm_aie_mem - The data for AIE memory read/write
 * @col:   The AIE column index
 * @row:   The AIE row index
 * @addr:  The AIE memory address to read/write
 * @size:  The size of bytes to read/write
 * @buf_p: The buffer to store read/write data
 *
 * This is used for DRM_AMDXDNA_READ_AIE_MEM and DRM_AMDXDNA_WRITE_AIE_MEM
 * parameters.
 */
struct amdxdna_drm_aie_mem {
	__u32 col;
	__u32 row;
	__u32 addr;
	__u32 size;
	__u64 buf_p;
};

/**
 * struct amdxdna_drm_aie_reg - The data for AIE register read/write
 * @col: The AIE column index
 * @row: The AIE row index
 * @addr: The AIE register address to read/write
 * @val: The value to write or returned value from AIE
 *
 * This is used for DRM_AMDXDNA_READ_AIE_REG and DRM_AMDXDNA_WRITE_AIE_REG
 * parameters.
 */
struct amdxdna_drm_aie_reg {
	__u32 col;
	__u32 row;
	__u32 addr;
	__u32 val;
};

/**
 * struct amdxdna_drm_get_power_mode - Get the power mode of the AIE hardware
 * @power_mode: Returned current power mode
 * @pad: MBZ.
 */
struct amdxdna_drm_get_power_mode {
	__u8 power_mode;
	__u8 pad[7];
};

/**
 * struct amdxdna_drm_query_firmware_version - Query the version of the firmware
 * @major: The major version number
 * @minor: The minor version number
 * @patch: The patch level version number
 * @build: The build ID
 */
struct amdxdna_drm_query_firmware_version {
	__u32 major; /* out */
	__u32 minor; /* out */
	__u32 patch; /* out */
	__u32 build; /* out */
};

/**
 * struct amdxdna_drm_get_force_preempt_state - Get force preemption state.
 * @force_preempt_state: 1 implies force preemption is enabled.
 *                       0 implies disabled.
 * @pad: MBZ.
 */
struct amdxdna_drm_get_force_preempt_state {
	__u8 state;
	__u8 pad[7];
};

/**
 * struct amdxdna_drm_get_info - Get some information from the AIE hardware.
 * @param: Specifies the structure passed in the buffer.
 * @buffer_size: Size of the input buffer. Size needed/written by the kernel.
 * @buffer: A structure specified by the param struct member.
 */
struct amdxdna_drm_get_info {
#define	DRM_AMDXDNA_QUERY_AIE_STATUS		0
#define	DRM_AMDXDNA_QUERY_AIE_METADATA		1
#define	DRM_AMDXDNA_QUERY_AIE_VERSION		2
#define	DRM_AMDXDNA_QUERY_CLOCK_METADATA	3
#define	DRM_AMDXDNA_QUERY_SENSORS		4
#define	DRM_AMDXDNA_QUERY_HW_CONTEXTS		5
#define	DRM_AMDXDNA_READ_AIE_MEM		6
#define	DRM_AMDXDNA_READ_AIE_REG		7
#define	DRM_AMDXDNA_QUERY_FIRMWARE_VERSION	8
#define	DRM_AMDXDNA_GET_POWER_MODE		9
#define	DRM_AMDXDNA_QUERY_TELEMETRY		10
#define	DRM_AMDXDNA_GET_FORCE_PREEMPT_STATE	11
	__u32 param; /* in */
	__u32 buffer_size; /* in/out */
	__u64 buffer; /* in/out */
};

/**
 * struct amdxdna_drm_set_power_mode - Set the power mode of the AIE hardware
 * @power_mode: The target power mode to be set
 * @pad: MBZ.
 */
struct amdxdna_drm_set_power_mode {
	__u8 power_mode;
	__u8 pad[7];
};

/**
 * struct amdxdna_drm_set_force_preempt_state - set force preemption state
 * @force_preempt_state: 1 implies force preemption is enabled.
 *                       0 implies disabled
 * @pad: MBZ.
 */
struct amdxdna_drm_set_force_preempt_state {
	__u8 state;
	__u8 pad[7];
};

/**
 * struct amdxdna_drm_set_state - Set the state of some component within the AIE hardware.
 * @param: Specifies the structure passed in the buffer.
 * @buffer_size: Size of the input buffer.
 * @buffer: A structure specified by the param struct member.
 */
struct amdxdna_drm_set_state {
#define	DRM_AMDXDNA_SET_POWER_MODE		0
#define	DRM_AMDXDNA_WRITE_AIE_MEM		1
#define	DRM_AMDXDNA_WRITE_AIE_REG		2
#define	DRM_AMDXDNA_SET_FORCE_PREEMPT		3
	__u32 param; /* in */
	__u32 buffer_size; /* in */
	__u64 buffer; /* in */
};

#define DRM_IOCTL_AMDXDNA_CREATE_CTX \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_CREATE_CTX, \
		 struct amdxdna_drm_create_ctx)

#define DRM_IOCTL_AMDXDNA_DESTROY_CTX \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_DESTROY_CTX, \
		 struct amdxdna_drm_destroy_ctx)

#define DRM_IOCTL_AMDXDNA_CONFIG_CTX \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_CONFIG_CTX, \
		 struct amdxdna_drm_config_ctx)

#define DRM_IOCTL_AMDXDNA_CREATE_BO \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_CREATE_BO, \
		 struct amdxdna_drm_create_bo)

#define DRM_IOCTL_AMDXDNA_GET_BO_INFO \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_GET_BO_INFO, \
		 struct amdxdna_drm_get_bo_info)

#define DRM_IOCTL_AMDXDNA_SYNC_BO \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_SYNC_BO, \
		 struct amdxdna_drm_sync_bo)

#define DRM_IOCTL_AMDXDNA_EXEC_CMD \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_EXEC_CMD, \
		 struct amdxdna_drm_exec_cmd)

#define DRM_IOCTL_AMDXDNA_WAIT_CMD \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_WAIT_CMD, \
		 struct amdxdna_drm_wait_cmd)

#define DRM_IOCTL_AMDXDNA_GET_INFO \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_GET_INFO, \
		 struct amdxdna_drm_get_info)

#define DRM_IOCTL_AMDXDNA_SET_STATE \
	DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_SET_STATE, \
		 struct amdxdna_drm_set_state)

#if defined(__cplusplus)
} /* extern c end */
#endif

#endif /* AMDXDNA_ACCEL_H_ */
