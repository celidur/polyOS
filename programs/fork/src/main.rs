#![no_main]
#![no_std]

use polyos_std::*;

#[polyos_std::main]
fn main() {
    let (pipe_read, pipe_write) = match polyos_std::io::pipe() {
        Ok(fds) => fds,
        Err(err) => {
            println!("pipe failed: {}", err);
            return;
        }
    };
    let semaphore = match polyos_std::sync::Semaphore::create(0) {
        Ok(semaphore) => semaphore,
        Err(err) => {
            println!("sem_create failed: {}", err);
            return;
        }
    };

    let mut stack_value = 0x11112222u32;
    let stack_ptr = &mut stack_value as *mut u32;

    println!("before fork: value={:x}", unsafe {
        core::ptr::read_volatile(stack_ptr)
    });

    let pid = polyos_std::process::fork();

    if pid == 0 {
        let _ = polyos_std::io::close(pipe_read);

        unsafe {
            core::ptr::write_volatile(stack_ptr, 0x33334444);
        }

        let message = b"hello from child through pipe";
        match polyos_std::io::write(pipe_write, message) {
            Ok(written) => println!("child: pipe wrote {} bytes", written),
            Err(err) => println!("child: pipe write failed: {}", err),
        }
        let _ = polyos_std::io::close(pipe_write);

        println!("child: fork returned 0, value={:x}", unsafe {
            core::ptr::read_volatile(stack_ptr)
        });

        match semaphore.signal() {
            Ok(()) => println!("child: semaphore signaled"),
            Err(err) => println!("child: semaphore signal failed: {}", err),
        }

        println!("child: waiting on semaphore {}", semaphore.id());
        match semaphore.wait() {
            Ok(()) => println!("child: semaphore acquired"),
            Err(err) => println!("child: semaphore wait failed: {}", err),
        }

        polyos_std::process::exit(0);
    }

    if pid > 0 {
        let _ = polyos_std::io::close(pipe_write);

        println!("parent: waiting on semaphore {}", semaphore.id());
        match semaphore.wait() {
            Ok(()) => println!("parent: semaphore acquired"),
            Err(err) => println!("parent: semaphore wait failed: {}", err),
        }

        match semaphore.signal() {
            Ok(()) => println!("parent: semaphore signaled"),
            Err(err) => println!("parent: semaphore signal failed: {}", err),
        }

        let mut status = -1;
        let waited = polyos_std::process::waitpid(pid, &mut status, 0);

        let mut pipe_buf = [0_u8; 64];
        let pipe_read_result = match polyos_std::io::read(pipe_read, &mut pipe_buf) {
            Ok(read) => {
                let message = core::str::from_utf8(&pipe_buf[..read]).unwrap_or("<invalid utf8>");
                println!("parent: pipe read {} bytes: {}", read, message);
                read as isize
            }
            Err(err) => {
                println!("parent: pipe read failed: {}", err);
                err
            }
        };
        let _ = polyos_std::io::close(pipe_read);
        let _ = semaphore.close();

        println!(
            "parent: child pid={}, waited={}, status={}, pipe_read={}, value={:x}",
            pid,
            waited,
            status,
            pipe_read_result,
            unsafe { core::ptr::read_volatile(stack_ptr) }
        );

        return;
    }

    println!("fork failed");
}
