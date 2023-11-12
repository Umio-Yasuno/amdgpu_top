#[cfg(feature = "git_version")]
fn get_head_id() -> Option<String> {
    use gix::ThreadSafeRepository;

    let g = ThreadSafeRepository::open("./.git").ok()?.to_thread_local();
    let head_id = g.head_id().ok()?.shorten().ok()?.to_string();

    Some(head_id)
}

#[cfg(not(feature = "git_version"))]
fn get_head_id() -> Option<String> {
    None
}

fn main() {
    const BASE: &str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

    let head_id = if let Some(head_id) = get_head_id() {
        format!(" (git-{head_id})")
    } else {
        String::new()
    };
    let debug = if cfg!(debug_assertions) { " (debug build)" } else { "" };
    let title = format!("{BASE}{head_id}{debug}");

    println!("cargo:rustc-env=TITLE={title}");
}
