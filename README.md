# amdgpu\_top
`amdgpu_top` is tool that show AMD GPU utilization, like [umr](https://gitlab.freedesktop.org/tomstdenis/umr/) or [clbr/radeontop](https://github.com/clbr/radeontop).  

![amdgpu_top screenshot](/docs/ss0.png)

## Usage
```
cargo run
```

### Command
| key |                                     |
| :-- | :---------------------------------: |
| g   | toggle GRBM                         |
| u   | toggle UVD                          |
| s   | toggle SRBM (SDMA, VCE)             |
| c   | toggle CP_STAT (Prefetch Parser, Micro Engine, Scratch Memory, ..) |
| v   | toggle VRAM/GTT Usage               |
| n   | toggle Sensors                      |
| q   | Quit                                |

## Library
 * [Cursive](https://github.com/gyscos/cursive)
 * [libdrm-amdgpu-sys-rs](https://github.com/Umio-Yasuno/libdrm-amdgpu-sys-rs)

## Reference
 * [Tom St Denis / umr · GitLab](https://gitlab.freedesktop.org/tomstdenis/umr/)
 * Mesa3D
    * [src/gallium/drivers/radeonsi/si_gpu_load.c · main · Mesa / mesa · GitLab](https://gitlab.freedesktop.org/mesa/mesa/-/blob/main/src/gallium/drivers/radeonsi/si_gpu_load.c)
 * AMD Documentation
    * [R6xx_R7xx_3D.pdf](https://developer.amd.com/wordpress/media/2013/10/R6xx_R7xx_3D.pdf)
    * [CIK_3D_registers_v2.pdf](http://developer.amd.com/wordpress/media/2013/10/CIK_3D_registers_v2.pdf)
 * Linux Kernel AMDGPU Driver
    * `amdgpu_allowed_register_entry`
        * `/drivers/gpu/drm/amd/amdgpu/{cik,nv,vi,si,soc15,soc21}.c`
    * Register Offset
        * `include/asic_reg/`

## TODO
 * Currently tested only on AMD Polaris11 GPU (Radeon RX 560)
 * RDNA 3 support
 * more commands
 * update theme
