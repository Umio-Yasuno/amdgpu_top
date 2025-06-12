# XDNA
## Generate bindings for xdna-driver
```
$ bindgen --no-layout-tests --wrap-unsafe-ops header/amdxdna_accel.h > bindings.rs
```

### Header source
<https://github.com/amd/xdna-driver/blob/main/src/include/uapi/drm_local/amdxdna_accel.h>
