use std::fs;
use std::time::Duration;
use std::path::Path;
use crate::DevicePath;

#[derive(Debug, Default, Clone)]
pub struct ProcInfo {
    pub pid: i32,
    pub name: String,
    pub fds: Vec<i32>,
}

fn get_fds<T: AsRef<Path>>(pid: i32, device_path: &[T]) -> Vec<i32> {
    let Ok(fd_list) = fs::read_dir(format!("/proc/{pid}/fd/")) else { return Vec::new() };

    fd_list.filter_map(|fd_link| {
        let dir_entry = fd_link.map(|fd_link| fd_link.path()).ok()?;
        let link = fs::read_link(&dir_entry).ok()?;

        // e.g. "/dev/dri/renderD128" or "/dev/dri/card0"
        if device_path.iter().any(|path| link.starts_with(path)) {
            dir_entry.file_name()?.to_str()?.parse::<i32>().ok()
        } else {
            None
        }
    }).collect()
}

pub fn get_all_processes() -> Vec<i32> {
    const SYSTEMD_CMDLINE: &[&str] = &[ "/lib/systemd", "/usr/lib/systemd" ];

    let Ok(proc_dir) = fs::read_dir("/proc") else { return Vec::new() };

    proc_dir.filter_map(|dir_entry| {
        let dir_entry = dir_entry.ok()?;
        let metadata = dir_entry.metadata().ok()?;

        if !metadata.is_dir() { return None }

        let pid = dir_entry.file_name().to_str()?.parse::<i32>().ok()?;

        if pid == 1 { return None } // init process, systemd

        // filter systemd processes from fdinfo target
        // gnome-shell share the AMDGPU driver context with systemd processes
        {
            let cmdline = fs::read_to_string(format!("/proc/{pid}/cmdline")).ok()?;
            if SYSTEMD_CMDLINE.iter().any(|path| cmdline.starts_with(path)) {
                return None;
            }
        }

        Some(pid)
    }).collect()
}

pub fn update_index_by_all_proc<T: AsRef<Path>>(
    vec_info: &mut Vec<ProcInfo>,
    device_path: &[T],
    all_proc: &[i32],
) {
    vec_info.clear();

    for p in all_proc {
        let pid = *p;
        let fds = get_fds(pid, device_path);

        if fds.is_empty() { continue }

        // Maximum 16 characters
        // https://www.kernel.org/doc/html/latest/filesystems/proc.html#proc-pid-comm-proc-pid-task-tid-comm
        let Ok(mut name) = fs::read_to_string(format!("/proc/{pid}/comm")) else { continue };
        name.pop(); // trim '\n'

        vec_info.push(ProcInfo { pid, name, fds });
    }
}

pub fn update_index(vec_info: &mut Vec<ProcInfo>, device_path: &DevicePath) {
    update_index_by_all_proc(
        vec_info,
        &[&device_path.render, &device_path.card],
        &get_all_processes(),
    );
}

pub fn spawn_update_index_thread(
    device_paths: Vec<DevicePath>,
    interval: u64,
) {
    let mut buf_index: Vec<ProcInfo> = Vec::new();
    let interval = Duration::from_secs(interval);

    std::thread::spawn(move || loop {
        let all_proc = get_all_processes();

        for device_path in &device_paths {
            update_index_by_all_proc(
                &mut buf_index,
                &[&device_path.render, &device_path.card],
                &all_proc,
            );

            let lock = device_path.arc_proc_index.lock();
            if let Ok(mut index) = lock {
                index.clone_from(&buf_index);
            }
        }

        std::thread::sleep(interval);
    });
}

pub fn diff_usage(pre: i64, cur: i64, interval: &Duration) -> i64 {
    use std::ops::Mul;

    let diff_ns = if pre == 0 || cur < pre {
        return 0;
    } else {
        cur.saturating_sub(pre) as u128
    };

    diff_ns
        .mul(100)
        .checked_div(interval.as_nanos())
        .unwrap_or(0) as i64
}
