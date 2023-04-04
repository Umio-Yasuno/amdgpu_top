# AMDGPU\_TOP
`amdgpu_top` is tool that display AMD GPU utilization, like [umr](https://gitlab.freedesktop.org/tomstdenis/umr/) or [clbr/radeontop](https://github.com/clbr/radeontop).  
The tool displays information gathered from performance counters (GRBM, GRBM2, CP_STAT), sensors, fdinfo, and AMDGPU driver.  

![amdgpu_top screenshot](/docs/ss0.png)

## Usage
```
cargo run -- [options ..]
```

### Option
```
FLAGS:
   -d, --dump
       Dump AMDGPU info (name, clock, chip_class, VRAM, PCI, VBIOS)

OPTIONS:
   -i <u32>
       Select GPU instance
```

### Command
| key |                                     |
| :-- | :---------------------------------: |
| g   | toggle GRBM                         |
| r   | toggle GRBM2                        |
| c   | toggle CP_STAT (Prefetch Parser, Micro Engine, Scratch Memory, ..) |
| p   | toggle PCI                          |
| v   | toggle VRAM/GTT Usage               |
| f   | toggle fdinfo                       |
| n   | toggle Sensors                      |
| h   | change update interval (high = 100ms, low = 1000ms) |
| q   | Quit                                |
| P   | sort fdinfo by pid                  |
| M   | sort fdinfo by VRAM usage           |
| G   | sort fdinfo by GFX usage            |
| R   | reverse sort                        |

## Installation
### Build from source
```
git clone https://github.com/Umio-Yasuno/amdgpu_top
cd amdgpu_top
cargo install --locked --path .
```

## Library
 * [Cursive](https://github.com/gyscos/cursive)
 * [libdrm-amdgpu-sys-rs](https://github.com/Umio-Yasuno/libdrm-amdgpu-sys-rs)
 * [procfs](https://github.com/eminence/procfs)

## Reference
 * [Tom St Denis / umr · GitLab](https://gitlab.freedesktop.org/tomstdenis/umr/)
 * Mesa3D
    * [src/gallium/drivers/radeonsi/si_gpu_load.c · main · Mesa / mesa · GitLab](https://gitlab.freedesktop.org/mesa/mesa/-/blob/main/src/gallium/drivers/radeonsi/si_gpu_load.c)
 * AMD Documentation
    * [R6xx_R7xx_3D.pdf](https://developer.amd.com/wordpress/media/2013/10/R6xx_R7xx_3D.pdf)
    * [CIK_3D_registers_v2.pdf](http://developer.amd.com/wordpress/media/2013/10/CIK_3D_registers_v2.pdf)
    * [MI200 Performance Counters: Listing](https://docs.amd.com/bundle/AMD-Instinct-MI200-Performance-Counters-v5.3/page/MI200_Performance_Counters_Listing.html)
    * [MI200 Performance Counters: Abbreviations](https://docs.amd.com/bundle/AMD-Instinct-MI200-Performance-Counters-v5.3/page/MI200_Performance_Counters_Abbreviations.html)
 * <https://github.com/AMDResearch/omniperf/tree/v1.0.4/src/perfmon_pub>
 * <https://github.com/freedesktop/mesa-r600_demo>
 * [radeonhd:r6xxErrata](https://www.x.org/wiki/radeonhd:r6xxErrata/)
 * Linux Kernel AMDGPU Driver
    * libdrm_amdgpu API
        * `/drivers/gpu/drm/amd/amdgpu/amdgpu_kms.c`
    * `amdgpu_allowed_register_entry`
        * `/drivers/gpu/drm/amd/amdgpu/{cik,nv,vi,si,soc15,soc21}.c`

## Note
 * Currently tested only on AMD Polaris11 GPU (Radeon RX 560)
 * Only `amdgpu_read_mm_registers` function is used to read registers.

## TODO
 * update theme
 * packages
 * config file?
 * logger?

## Alternatives
If `amdgpu_top` is not enough for you or you don't like it, try the following applications.

 * [clbr/radeontop](https://github.com/clbr/radeontop)
    * View your GPU utilization, both for the total activity percent and individual blocks.
 * [Syllo/nvtop](https://github.com/Syllo/nvtop)
    * GPUs process monitoring for AMD, Intel and NVIDIA
 * [Tom St Denis / umr · GitLab](https://gitlab.freedesktop.org/tomstdenis/umr/)
    * User Mode Register Debugger for AMDGPU Hardware
 * [GPUOpen-Tools/radeon_gpu_profiler](https://github.com/GPUOpen-Tools/radeon_gpu_profiler)
    * for developer
    * Radeon GPU Profiler (RGP) is a tool from AMD that allows for deep inspection of GPU workloads. 
