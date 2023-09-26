% AMDGPU_TOP(1)
% Umio Yasuno <coelacanth_dream@protonmail.com>
% 2023-07-16

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

**Dump info for all AMDGPU devices**

    $ amdgpu_top --list -d

**Specifies */dev/dri/renderD129* **

    $ amdgpu_top -i 1

**Specifies PCI bus**

    $ amdgpu_top --pci "0000:01:00.0"

# OPTIONS
**\-i** *`<u32>`*
:   Select GPU instance

**\-\-pci** *`<String>`*
:   Specifying PCI path (domain:bus:dev.func)

**-s** *`<u64>`*, **-s** *`<u64>ms`*
: Refresh period in milliseconds for JSON mode (default: 1000ms)

**-u** *`<u64>`*, **--update-process-index** *`<u64>`*
: Update interval in seconds of the process index for fdinfo (default: 5s)

**\--apu**, **\-\-select-apu**
:   Select APU instance

**\-d**, **\-\-dump**
:   Dump AMDGPU info (Specifications, VRAM, PCI, ResizableBAR, VBIOS, Video caps) (can be combined with "-J" option)

**\-\-list**
:   Display a list of AMDGPU devices (can be combined with "-d" option)

**\-J**, **\-\-json**
:   Output JSON formatted data

**\-\-gui**
:   Launch GUI mode

**\-\-smi**
:   Launch Simple TUI mode (like nvidia-smi, rocm-smi)

**\-h**, **\-\-help**
:   Print help information

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
