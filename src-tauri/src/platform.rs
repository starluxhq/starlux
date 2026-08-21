#[cfg(target_os = "linux")]
pub fn prepare_graphics() {
    use std::env;

    let set_if_unset = |key: &str, value: &str| {
        if env::var_os(key).is_none() {
            env::set_var(key, value);
        }
    };

    let safe_mode = env::var_os("STARLUX_SAFE_GRAPHICS").is_some();

    if env::var_os("STARLUX_FORCE_X11").is_some() {
        set_if_unset("GDK_BACKEND", "x11");
    }

    set_if_unset("__NV_DISABLE_EXPLICIT_SYNC", "1");

    if safe_mode || std::path::Path::new("/proc/driver/nvidia").exists() {
        set_if_unset("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    if safe_mode {
        set_if_unset("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }
}

#[cfg(not(target_os = "linux"))]
pub fn prepare_graphics() {}
