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
    let head_id = get_head_id().unwrap_or(String::new());
    println!("cargo:rustc-env=HEAD_ID={head_id}");
}
