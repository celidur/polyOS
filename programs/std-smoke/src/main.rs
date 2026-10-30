fn main() {
    println!("rust-std: hello from PolyOS");
    println!("rust-std: args={:?}", std::env::args().collect::<Vec<_>>());
    println!("rust-std: cwd={:?}", std::env::current_dir());
    let mut map = std::collections::HashMap::new();
    map.insert("PolyOS", 42);
    assert_eq!(map["PolyOS"], 42);
    std::fs::write("/tmp/std-smoke.txt", "std filesystem\n").unwrap();
    assert_eq!(
        std::fs::read_to_string("/tmp/std-smoke.txt").unwrap(),
        "std filesystem\n"
    );
    assert!(std::fs::read_dir("/tmp")
        .unwrap()
        .any(|e| e.unwrap().file_name() == "std-smoke.txt"));
    std::fs::remove_file("/tmp/std-smoke.txt").unwrap();
    println!("rust-std: filesystem and hashing passed");
    let state = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let other = state.clone();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let (lock, ready) = &*other;
        *lock.lock().unwrap() = true;
        ready.notify_one();
        73
    });
    let (lock, ready) = &*state;
    let (done, timeout) = ready
        .wait_timeout_while(
            lock.lock().unwrap(),
            std::time::Duration::from_secs(1),
            |done| !*done,
        )
        .unwrap();
    assert!(*done && !timeout.timed_out());
    drop(done);
    assert_eq!(thread.join().unwrap(), 73);
    println!("rust-std: threads and timed synchronization passed");
    let status = std::process::Command::new("/bin/selftest.elf")
        .arg("--thread-exec-child")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(43));
    let error = std::process::Command::new("/missing-std-program")
        .status()
        .unwrap_err();
    assert_eq!(error.raw_os_error(), Some(2));
    println!("rust-std: process spawn passed");
    println!("rust-std: all checks passed");
}
