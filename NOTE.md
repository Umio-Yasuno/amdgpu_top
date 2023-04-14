## Note
 * `amdgpu_top` can be selected for AMD GPUs not connected to the display, but register reads may cause frequent changes in the gfxoff state. It may have a negative impact on power consumption and display/audio output.
 * On APUs with LPDDR5 memory (e.g. VanGogh, Rembrandt/Yellow Carp), the memory bus width may be displayed twice as wide as it should be.
   * <https://gitlab.freedesktop.org/drm/amd/-/issues/2468>
 * Some AMD GPUs (GFX9 and later?) have some of the CP_STAT bits flipped.
   * <https://gitlab.freedesktop.org/drm/amd/-/issues/2512>
