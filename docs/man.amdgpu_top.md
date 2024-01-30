% AMDGPU_TOP(1)
% Umio Yasuno <coelacanth_dream@protonmail.com>
% 2023-12-01

<!-- $ pandoc docs/man.amdgpu_top.md -s -t man -o docs/amdgpu_top.1 -->

# NAME

amdgpu_top - Tool to displays AMDGPU usage.

# SYNOPSIS

*amdgpu_top* [*OPTIONS*]

# DESCRIPTION

*amdgpu_top* is tool that display AMD GPU utilization, like *umr* [^1] or *clbr/radeontop* [^2]  or *intel_gpu_top* [^3] .  
The tool displays information gathered from performance counters (GRBM, GRBM2), sensors, fdinfo, and AMDGPU driver.

[^1]: <https://gitlab.freedesktop.org/tomstdenis/umr/>
[^2]: <https://github.com/clbr/radeontop>
[^3]: <https://gitlab.freedesktop.org/drm/igt-gpu-tools/-/blob/master/man/intel_gpu_top.rst>

# EXAMPLES
**Display a list of AMDGPU devices**

    $ amdgpu_top --list

**Dump All GPU processes and memory usage per process**

    $ amdgpu_top -p

**Specifies PCI bus**

    $ amdgpu_top --pci "0000:01:00.0"

# OPTIONS
**\-i** *`<usize>`*
:   Select GPU instance.

**\-\-pci** *`<String>`*
:   Specifying PCI path. (domain:bus:dev.func)

**-s** *`<u64>`*, **-s** *`<u64>ms`*
:   Refresh period (interval) in milliseconds for JSON mode. (default: 1000ms)

**-n** *`<u32>`*
:   Specifies the maximum number of iteration for JSON mode. If 0 is specified, it will be an infinite loop. (default: 0)

**-u** *`<u64>`*, **\-\-update-process-index** *`<u64>`*
:   Update interval in seconds of the process index for fdinfo. (default: 5s)
**\-\-json_fifo** *`<String>`*, **\-\-json-fifo** *`<String>`*
:   Output JSON formatted data to FIFO (named pipe) for other application and scripts.

**\-\-apu**, **\-\-select-apu**
:   Select APU instance.

**\-\-single**, **\-\-single-gpu**
:   Display only the selected GPU/APU.

**\-\-no\-pc**
:   The application does not read the performance counter (GRBM, GRBM2) if this flag is set. Reading the performance counter may deactivate the power saving feature of APU/GPU.

**\-gm**, **\-\-gpu_metrics**, **\-\-gpu-metrics**
:   Dump gpu_metrics for all AMD GPUs. https://www.kernel.org/doc/html/latest/gpu/amdgpu/thermal.html#gpu-metrics

**\-\-drm_info**, **\-\-drm-info**
:   Dump DRM info. Inspired by https://gitlab.freedesktop.org/emersion/drm_info

**\-\-dark**, **\-\-dark-mode**
:   Set to the dark mode. (TUI/GUI)

**\-\-light**, **\-\-light-mode**
:   Set to the light mode. (TUI/GUI)

**\-d**, **\-\-dump**
:   Dump AMDGPU info. (Specifications, VRAM, PCI, ResizableBAR, VBIOS, Video caps) This option can be combined with the "-J" option.

**\-\-list**
:   Display a list of AMDGPU devices.

**\-p**, **\-\-process**
:   Dump All GPU processes and memory usage per process.

**\-J**, **\-\-json**
:   Output JSON formatted data.  This option can be combined with the "-d" option.

**\-\-gui**
:   Launch GUI mode.

**\-\-smi**
:   Launch Simple TUI mode. (like nvidia-smi, rocm-smi)

**\-V**, **\-\-version**
:   Print version information.

**\-h**, **\-\-help**
:   Print help information.

# COMMANDS FOR TUI MODE
| key |                                     |
| :-- | :---------------------------------- |
| f   | toggle fdinfo                       |
| n   | toggle Sensors                      |
| m   | toggle GPU Metrics                  |
| h   | change update interval (high = 100ms, low = 1000ms) |
| q   | Quit                                |
| P   | sort fdinfo by pid                  |
| M   | sort fdinfo by VRAM usage           |
| G   | sort fdinfo by GFX usage            |
| M   | sort fdinfo by MediaEngine usage    |
| R   | reverse sort for fdinfo             |

# BUGS
<https://github.com/Umio-Yasuno/amdgpu_top/issues>
