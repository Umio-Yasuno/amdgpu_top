# XDNA
## Generate bindings
```
$ cd <linux_kernel>
$ bindgen --no-layout-tests --wrap-unsafe-ops include/uapi/drm/amdxdna_accel.h -- -D__user= > <amdgpu_top_dir>/crates/libamdgpu_top/src/xdna/bindings.rs
```
