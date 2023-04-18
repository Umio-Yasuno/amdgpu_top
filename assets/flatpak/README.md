## Flatpak
### Installation
```
git clone https://github.com/Umio-Yasuno/amdgpu_top
cd amdgpu_top/assets/flatpak
flatpak-builder --install repo org.umioyasuno.amdgpu_top.json --user --force-clean
```

### Note

 * Flatpak dose not allow direct access to `/proc`, so `amdgpu_top` can't get GPU utilization (VRAM, GFX, Compute, Decode, Encode) from `/proc/<PID>/fdinfo`.
    * <https://docs.flatpak.org/en/latest/sandbox-permissions.html#filesystem-access>
 * I recommend using .AppImage package instead of Flatpak package.
    * <https://github.com/Umio-Yasuno/amdgpu_top/releases/tag/v0.1.4>
