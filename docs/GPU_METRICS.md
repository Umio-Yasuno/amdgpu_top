## GPU Metrics
Vega12 or later (dGPU), Renoir or later (APU) supports it.  
GPU Metrics include temperature, frequency, engines utilization, power consume, throttler status, fan speed, CPU core and CPU L3 cache statistics.

 * v1.0: Edge/Hotspot/Mem/VRGFX/VRSoC/VRMem, Average Socket Power
 * v1.1/v1.2: Edge/Hotspot/Mem/VRGFX/VRSoC/VRMem/HBM Temperature, Average Socket Power
 * v1.3: Edge/Hotspot/Mem/VRGFX/VRSoC/VRMem/HBM Temperature, Average Socket Power, SoC/GFX/Mem Voltage
 * v2.1/v2.2/v2.3: GFX/SoC/CPU Core/L3Cache Temperature, Average Socket/CPU/SoC/GFX/CPU Core Power

### Note
 * Only Aldebaran (MI200) supports "HBM Temperature".
 * Renoir, Lucienne, Cezanne (Green Sardine), Barcelo APU dose not support "Average GFX Power".

### Reference
 * <https://github.com/torvalds/linux/blob/master/drivers/gpu/drm/amd/include/kgd_pp_interface.h>
 * <https://github.com/torvalds/linux/blob/master/drivers/gpu/drm/amd/pm/amdgpu_pm.c>
