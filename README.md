# AMDGPU\_TOP
`amdgpu_top` is tool that display AMD GPU utilization, like [umr](https://gitlab.freedesktop.org/tomstdenis/umr/) or [clbr/radeontop](https://github.com/clbr/radeontop) or [intel_gpu_top](https://gitlab.freedesktop.org/drm/igt-gpu-tools/-/blob/master/man/intel_gpu_top.rst).  
The tool displays information gathered from performance counters (GRBM, GRBM2), sensors, fdinfo, and AMDGPU driver.  

| Simple TUI<br>(like nvidia-smi, rocm-smi) | TUI | GUI |
| :-: | :-: | :-: |
| ![amdgpu_top Simple TUI](https://github.com/Umio-Yasuno/amdgpu_top/assets/53935716/66c3fb7e-cb23-4a19-ab10-2bb9919a1a8a) | ![amdgpu_top TUI](https://github.com/Umio-Yasuno/amdgpu_top/assets/53935716/859010d8-07b3-411c-b079-c4a837855d41) | ![amdgpu_top GUI mode](https://github.com/Umio-Yasuno/amdgpu_top/assets/53935716/e3ff372e-86f9-4b82-b3c9-910b638a3c90) |

## Quick links
 * [Usage](#usage)
   * [Options](#options)
   * [Commands for TUI](#commands-for-tui)
   * [Example of using JSON mode](#example-of-using-json-mode)
 * [Installation](#installation)
   * [Packages](#packages)
   * [Build from source](#build-from-source)
     * [Distribution specific instructions](#distribution-specific-instructions)
   * [Binary Size](#binary-size)
 * [References](#references)
 * [Translations](#translations)
 * [Alternatives](#alternatives)

## Dependent dynamic libraries
 * libdrm
 * libdrm_amdgpu

## Usage
```
cargo run -- [options ..]
# or
amdgpu_top [options ..]
```

### Options
```
FLAGS:
   -d, --dump
       Dump AMDGPU info. (Specifications, VRAM, PCI, ResizableBAR, VBIOS, Video caps)
       This option can be combined with the "-J" option.
   --list
       Display a list of AMDGPU devices.
   -J, --json
       Output JSON formatted data.
       This option can be combined with the "-d" option.
   --gui
       Launch GUI mode.
   --smi
       Launch Simple TUI mode. (like nvidia-smi, rocm-smi)
   -p, --process
       Dump All GPU processes and memory usage per process.
   --apu, --select-apu
       Select APU instance.
   --single, --single-gpu
       Display only the selected APU/GPU
   --no-pc
       The application does not read the performance counter (GRBM, GRBM2)
       if this flag is set.
       Reading the performance counter may deactivate the power saving feature of APU/GPU.
   -gm, --gpu_metrics, --gpu-metrics
       Dump gpu_metrics for all AMD GPUs.
       https://www.kernel.org/doc/html/latest/gpu/amdgpu/thermal.html#gpu-metrics
   --pp_table, --pp-table
       Dump pp_table from sysfs and VBIOS for all AMD GPUs.
       (only support Navi1x and Navi2x, Navi3x)
   --drm_info, --drm-info
       Dump DRM info.
       Inspired by https://gitlab.freedesktop.org/emersion/drm_info
   --dark, --dark-mode
       Set to the dark mode. (TUI/GUI)
   --light, --light-mode
       Set to the light mode. (TUI/GUI)
   -V, --version
       Print version information.
   -h, --help
       Print help information.

OPTIONS:
   -i <usize>
       Select GPU instance.
   --pci <String>
       Specifying PCI path. (domain:bus:dev.func)
   -s <u64>, -s <u64>ms
       Refresh period (interval) in milliseconds for JSON mode. (default: 1000ms)
   -n <u32>
       Specifies the maximum number of iteration for JSON mode.
       If 0 is specified, it will be an infinite loop. (default: 0)
   -u <u64>, --update-process-index <u64>
       Update interval in seconds of the process index for fdinfo. (default: 5s)
   --json_fifo, --json-fifo <String>
       Output JSON formatted data to FIFO (named pipe) for other application and scripts.
```

### Commands for TUI
| key |                                     |
| :-- | :---------------------------------: |
| g   | toggle GRBM                         |
| r   | toggle GRBM2                        |
| v   | toggle VRAM/GTT Usage               |
| f   | toggle fdinfo                       |
| n   | toggle Sensors                      |
| m   | toggle GPU Metrics                  |
| h   | change update interval (high = 100ms, low = 1000ms) |
| q   | Quit                                |
| P   | sort fdinfo by pid                  |
| M   | sort fdinfo by VRAM usage           |
| G   | sort fdinfo by GFX usage            |
| M   | sort fdinfo by MediaEngine usage    |
| R   | reverse sort                        |

### Example of using JSON mode
```
$ amdgpu_top --json | jq -c -r '(.devices[] | (.Info | .DeviceName + " (" + .PCI + "): ") + ([.gpu_activity | to_entries[] | .key + ": " + (.value.value|tostring) + .value.unit] | join(", ")))'
AMD Radeon RX 6600 (0000:03:00.0): GFX: 13%, MediaEngine: 0%, Memory: 4%
AMD Radeon Graphics (0000:08:00.0): GFX: 0%, MediaEngine: 0%, Memory: null%
AMD Radeon RX 6600 (0000:03:00.0): GFX: 15%, MediaEngine: 0%, Memory: 5%
AMD Radeon Graphics (0000:08:00.0): GFX: 0%, MediaEngine: 0%, Memory: null%
AMD Radeon RX 6600 (0000:03:00.0): GFX: 3%, MediaEngine: 0%, Memory: 2%
AMD Radeon Graphics (0000:08:00.0): GFX: 0%, MediaEngine: 0%, Memory: null%
...
```

## Installation
### Packages
 * [Releases](https://github.com/Umio-Yasuno/amdgpu_top/releases/latest)
   * .deb (generated by [cargo-deb](https://github.com/kornelski/cargo-deb))
   * .rpm (generated by [cargo-generate-rpm](https://github.com/cat-in-136/cargo-generate-rpm))
   * .AppImage (generated by [cargo-appimage](https://github.com/StratusFearMe21/cargo-appimage))
 * AUR
   * [amdgpu_top](https://aur.archlinux.org/packages/amdgpu_top)
   * [amdgpu_top-bin](https://aur.archlinux.org/packages/amdgpu_top-bin)
   * [amdgpu_top-git](https://aur.archlinux.org/packages/amdgpu_top-git)
 * [OpenMandriva](https://github.com/OpenMandrivaAssociation/amdgpu_top) to install run `sudo dnf install amdgpu_top`
 * [Nix](https://github.com/NixOS/nixpkgs/blob/master/pkgs/tools/system/amdgpu_top/default.nix)
 * [Solus](https://github.com/getsolus/packages/tree/main/packages/a/amdgpu_top) to install run `sudo eopkg it amdgpu_top`

### Build from source
```
cargo install amdgpu_top

# or

git clone https://github.com/Umio-Yasuno/amdgpu_top
cd amdgpu_top
cargo install --locked --path .
```

#### without GUI
```
cargo install --locked --path . --no-default-features --features="tui"
```

#### Distribution specific instructions
##### Debian/Ubuntu
```
sudo apt install libdrm-dev
```

### Binary Size

| Features       | Size (stripped) |
| :------------- | :-------------: |
| json           | ~852K |
| tui            | ~1.3M |
| json, tui      | ~1.4M |
| json, tui, gui | ~14M  |

## References
 * [Tom St Denis / umr · GitLab](https://gitlab.freedesktop.org/tomstdenis/umr/)
 * Mesa3D
    * [src/gallium/drivers/radeonsi/si_gpu_load.c · main · Mesa / mesa · GitLab](https://gitlab.freedesktop.org/mesa/mesa/-/blob/main/src/gallium/drivers/radeonsi/si_gpu_load.c)
 * AMD Documentation
    * [R6xx_R7xx_3D.pdf](https://developer.amd.com/wordpress/media/2013/10/R6xx_R7xx_3D.pdf)
    * [CIK_3D_registers_v2.pdf](http://developer.amd.com/wordpress/media/2013/10/CIK_3D_registers_v2.pdf)
    * [MI200 performance counters and metrics — ROCm Documentation](https://rocm.docs.amd.com/en/docs-6.0.0/conceptual/gpu-arch/mi200-performance-counters.html)
 * <https://github.com/AMDResearch/omniperf/tree/v1.0.4/src/perfmon_pub>
 * <https://github.com/freedesktop/mesa-r600_demo>
 * [radeonhd:r6xxErrata](https://www.x.org/wiki/radeonhd:r6xxErrata/)
 * Linux Kernel AMDGPU Driver
    * libdrm_amdgpu API
        * `/drivers/gpu/drm/amd/amdgpu/amdgpu_kms.c`
    * `amdgpu_allowed_register_entry`
        * `/drivers/gpu/drm/amd/amdgpu/{cik,nv,vi,si,soc15,soc21}.c`

## Translations
`amdgpu_top` is using [cargo-i18n](https://github.com/kellpossible/cargo-i18n/) with [Project Fluent](https://projectfluent.org/) for translation.  
Please refer to [pop-os/popsicle](https://github.com/pop-os/popsicle#translators) for additional supported languages.  

### Supported Languages
 * [en](./crates/amdgpu_top_gui/i18n/en/amdgpu_top_gui.ftl)
 * [ja (partial)](./crates/amdgpu_top_gui/i18n/ja/amdgpu_top_gui.ftl)

## Alternatives
If `amdgpu_top` is not enough for you or you don't like it, try the following applications.

 * `AMD_DEBUG=info <opengl application>` or `RADV_DEBUG=info <vulkan application>`
    * Print AMDGPU-related information
    * <https://docs.mesa3d.org/envvars.html#envvar-AMD_DEBUG>
    * <https://docs.mesa3d.org/envvars.html#envvar-RADV_DEBUG>
 * [clbr/radeontop](https://github.com/clbr/radeontop)
    * View your GPU utilization, both for the total activity percent and individual blocks.
 * [Syllo/nvtop](https://github.com/Syllo/nvtop)
    * GPUs process monitoring for AMD, Intel and NVIDIA
 * [Tom St Denis / umr · GitLab](https://gitlab.freedesktop.org/tomstdenis/umr/)
    * User Mode Register Debugger for AMDGPU Hardware
 * [GPUOpen-Tools/radeon_gpu_profiler](https://github.com/GPUOpen-Tools/radeon_gpu_profiler)
    * for developer
    * Radeon GPU Profiler (RGP) is a tool from AMD that allows for deep inspection of GPU workloads. 
