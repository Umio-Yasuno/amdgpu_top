# Changelog
## 0.11.0 (2025-09-02)
 * fix Appstream metainfo by @malfisya
 * fix the process to get gpu_metrics when resuming from suspended state
 * stop monitoring when no VRAM-using processes are found
 * fix getting sensors when resuming monitoring
 * stop reading PC (GRBM, GRBM2) for RDNA 4 dGPU when no VRAM-using processes are found
 * add exclude process names (`amdgpu_top`, `steamwebhelper`) for no_process_using_vram
 * add memory_vendor to AppDeviceInfo
 * fix DevicePath::init when libdrm_amdgpu is None (#133)
 * fix combination of `--single/--single-gpu` and `--pci` options (#134)
 * reorder the device path list if a device is specified in the options
 * add support for FCLK (Fabric Clock) DPM
 * update vram usage when pre-dropping device handle
 * update logic for dropping device handle
 * add supports_gpu_metrics field to AppDeviceInfo
 * fix get_rocm_version for TheRock
 * remove dependency on anyhow

### XDNA
 * update bindings for 32bit targets

### TUI
 * do not display average_socket_power if gpu_metrics_v3_0
 * clear gpu_metrics_view when metrics is None
 * check stapm_limit and current_stapm_limit

### SMI
 * print the suspended state
 * print PCI power state
 * set FdInfoSortType::GTT when the device is APU
 * print junction temp instead of edge temp, if junction temp is available
 * add memory temp.
 * remove ECC status
 * update layout for the suspending device

### GUI
 * do not display average_socket_power if gpu_metrics_v3_0
 * add tab_gui mode
 * set striped to side panel

### JSON
 * add some info to json_info dump (#137)
