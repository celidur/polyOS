#![no_main]
#![no_std]

use core::ffi::c_void;

use bindings::{clear_screen, malloc, print_memory, reboot};
use polyos_std::*;

#[polyos_std::main]
fn main() {
    let mut buffer = [0u8; 1024];
    let mut env = ShellEnv::from_process();
    println!("PolyOS v2.0.0");
    loop {
        print!("> ");
        buffer.fill(0);
        let len = polyos_std::stdio::terminal_readline(&mut buffer, true);
        let buffer = core::str::from_utf8(&buffer[..len]).unwrap();
        println!();
        if buffer.is_empty() {
            continue;
        }

        let commands = match parse_command_line(buffer) {
            Ok(commands) => commands,
            Err(error) => {
                println!("{}", error);
                continue;
            }
        };

        if commands.len() == 1 && !commands[0].has_redirection() {
            if run_simple_command(&commands[0], &mut env) {
                break;
            }
        } else if let Err(error) = run_pipeline(&commands, &env) {
            println!("shell: {}", error);
        }
    }
}

#[derive(Clone, Copy)]
enum Redirect<'a> {
    Truncate(&'a str),
    Append(&'a str),
}

struct ParsedCommand<'a> {
    args: Vec<&'a str>,
    stdin: Option<&'a str>,
    stdout: Option<Redirect<'a>>,
}

impl ParsedCommand<'_> {
    fn command(&self) -> Option<&str> {
        self.args.first().copied()
    }

    fn has_redirection(&self) -> bool {
        self.stdin.is_some() || self.stdout.is_some()
    }
}

enum BuiltinResult {
    Continue(i32),
    Exit,
}

struct RedirectionError<'a> {
    path: &'a str,
    error: i32,
}

#[derive(Clone)]
struct ShellEnv {
    entries: Vec<String>,
}

impl ShellEnv {
    fn from_process() -> Self {
        let mut entries = Vec::new();
        let mut index = 0;
        while let Some(entry) = process::env(index) {
            entries.push(entry.to_string());
            index += 1;
        }

        let mut env = Self { entries };
        if env.get("PATH").is_none() {
            env.set("PATH", "/bin");
        }
        env
    }

    fn get(&self, name: &str) -> Option<&str> {
        for entry in self.entries.iter() {
            if let Some((entry_name, value)) = entry.split_once('=')
                && entry_name == name
            {
                return Some(value);
            }
        }
        None
    }

    fn set(&mut self, name: &str, value: &str) {
        let entry = format!("{}={}", name, value);
        for existing in self.entries.iter_mut() {
            if let Some((entry_name, _)) = existing.split_once('=')
                && entry_name == name
            {
                *existing = entry;
                return;
            }
        }
        self.entries.push(entry);
    }

    fn set_assignment(&mut self, assignment: &str) -> Result<(), &'static str> {
        let Some((name, value)) = assignment.split_once('=') else {
            return Err("export: expected NAME=VALUE");
        };

        if !valid_env_name(name) {
            return Err("export: invalid name");
        }

        self.set(name, value);
        Ok(())
    }

    fn as_strs(&self) -> Vec<&str> {
        self.entries.iter().map(|entry| entry.as_str()).collect()
    }
}

fn parse_command_line(input: &str) -> Result<Vec<ParsedCommand<'_>>, &'static str> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let mut commands = Vec::new();
    let mut current = ParsedCommand {
        args: Vec::new(),
        stdin: None,
        stdout: None,
    };

    let mut index = 0;
    while index < tokens.len() {
        match tokens[index] {
            "|" => {
                if current.args.is_empty() {
                    return Err("syntax error near '|'");
                }
                commands.push(current);
                current = ParsedCommand {
                    args: Vec::new(),
                    stdin: None,
                    stdout: None,
                };
            }
            "<" => {
                index += 1;
                let Some(path) = tokens.get(index).copied() else {
                    return Err("syntax error: missing input file");
                };
                current.stdin = Some(path);
            }
            ">" => {
                index += 1;
                let Some(path) = tokens.get(index).copied() else {
                    return Err("syntax error: missing output file");
                };
                current.stdout = Some(Redirect::Truncate(path));
            }
            ">>" => {
                index += 1;
                let Some(path) = tokens.get(index).copied() else {
                    return Err("syntax error: missing output file");
                };
                current.stdout = Some(Redirect::Append(path));
            }
            token => current.args.push(token),
        }
        index += 1;
    }

    if current.args.is_empty() {
        return Err("syntax error: missing command");
    }
    commands.push(current);
    Ok(commands)
}

fn run_simple_command(command: &ParsedCommand, env: &mut ShellEnv) -> bool {
    if let Some(result) = run_builtin(command, env) {
        return matches!(result, BuiltinResult::Exit);
    }

    let status = run_external_command(command, env);
    if status < 0 || status == 127 {
        if let Some(name) = command.command() {
            println!("{}: command not found", name);
        }
    } else {
        println!("Process exited with status {}", status);
    }

    false
}

fn run_external_command(command: &ParsedCommand, env: &ShellEnv) -> i32 {
    let pid = process::fork();
    if pid < 0 {
        return -1;
    }

    if pid == 0 {
        exec_search(command, env);
    }

    let mut status = 0;
    if process::waitpid(pid, &mut status, 0) < 0 {
        return -1;
    }

    (status >> 8) & 0xff
}

fn run_builtin(command: &ParsedCommand, env: &mut ShellEnv) -> Option<BuiltinResult> {
    let name = command.command()?;
    let args = &command.args[1..];
    match name {
        "help" => {
            print_help();
            Some(BuiltinResult::Continue(0))
        }
        "pwd" => {
            print_pwd();
            Some(BuiltinResult::Continue(0))
        }
        "cd" => {
            change_directory(args.first().copied().unwrap_or("/"));
            Some(BuiltinResult::Continue(0))
        }
        "ls" => {
            list_paths(args);
            Some(BuiltinResult::Continue(0))
        }
        "cat" => {
            cat_files(args);
            Some(BuiltinResult::Continue(0))
        }
        "touch" => {
            touch_files(args);
            Some(BuiltinResult::Continue(0))
        }
        "cp" => {
            copy_command(args);
            Some(BuiltinResult::Continue(0))
        }
        "mv" => {
            move_command(args);
            Some(BuiltinResult::Continue(0))
        }
        "mkdir" => {
            make_directories(args);
            Some(BuiltinResult::Continue(0))
        }
        "rmdir" => {
            remove_directories(args);
            Some(BuiltinResult::Continue(0))
        }
        "rm" => {
            remove_files(args);
            Some(BuiltinResult::Continue(0))
        }
        "stat" => {
            stat_paths(args);
            Some(BuiltinResult::Continue(0))
        }
        "echo" => {
            print_joined(args);
            Some(BuiltinResult::Continue(0))
        }
        "env" => {
            print_env(env);
            Some(BuiltinResult::Continue(0))
        }
        "export" => {
            export_env(env, args);
            Some(BuiltinResult::Continue(0))
        }
        "memory" => {
            unsafe { print_memory() };
            Some(BuiltinResult::Continue(0))
        }
        "exit" => Some(BuiltinResult::Exit),
        "malloc" => {
            let ptr = unsafe { malloc(4096 * 4096) };
            println!("malloc: {:x}", ptr as u32);
            Some(BuiltinResult::Continue(0))
        }
        "clear" => {
            unsafe { clear_screen() };
            Some(BuiltinResult::Continue(0))
        }
        "winsize" => {
            print_winsize();
            Some(BuiltinResult::Continue(0))
        }
        "devtest" => {
            devtest();
            Some(BuiltinResult::Continue(0))
        }
        "net" => {
            print_network_info();
            Some(BuiltinResult::Continue(0))
        }
        "dhcp" => {
            send_dhcp_discover();
            Some(BuiltinResult::Continue(0))
        }
        "ping" => {
            if let Some(target) = args.first().copied() {
                if let Some(ip) = parse_ipv4(target) {
                    ping_ipv4(ip);
                } else {
                    ping_name(target);
                }
            } else {
                ping_gateway();
            }
            Some(BuiltinResult::Continue(0))
        }
        "dns" => {
            if let Some(name) = args.first().copied() {
                send_dns_query(name);
            } else {
                println!("Usage: dns <name>");
            }
            Some(BuiltinResult::Continue(0))
        }
        "reboot" => {
            unsafe { reboot(bindings::RB_AUTOBOOT as i32) };
            Some(BuiltinResult::Continue(0))
        }
        "shutdown" => {
            unsafe { reboot(bindings::RB_POWER_OFF as i32) };
            Some(BuiltinResult::Continue(0))
        }
        _ => None,
    }
}

fn print_joined(args: &[&str]) {
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            print!(" ");
        }
        print!("{}", arg);
    }
    println!();
}

fn run_pipeline(commands: &[ParsedCommand], env: &ShellEnv) -> Result<(), &'static str> {
    let mut previous_read = -1;
    let mut pids = Vec::new();

    for (index, command) in commands.iter().enumerate() {
        let is_last = index + 1 == commands.len();
        let pipe = if is_last {
            None
        } else {
            Some(io::pipe().map_err(|_| "pipe failed")?)
        };

        let pid = process::fork();
        if pid < 0 {
            close_if_open(previous_read);
            if let Some((read_fd, write_fd)) = pipe {
                close_if_open(read_fd);
                close_if_open(write_fd);
            }
            return Err("fork failed");
        }

        if pid == 0 {
            if previous_read >= 0 {
                let _ = io::dup2(previous_read, bindings::STDIN_FILENO as i32);
            }

            if let Some((_read_fd, write_fd)) = pipe {
                let _ = io::dup2(write_fd, bindings::STDOUT_FILENO as i32);
            }

            close_if_open(previous_read);
            if let Some((read_fd, write_fd)) = pipe {
                close_if_open(read_fd);
                close_if_open(write_fd);
            }

            if let Err(error) = apply_redirections(command) {
                print_redirection_error(&error);
                process::exit(1);
            }

            let mut child_env = env.clone();
            if let Some(result) = run_builtin(command, &mut child_env) {
                let status = match result {
                    BuiltinResult::Continue(status) => status,
                    BuiltinResult::Exit => 0,
                };
                process::exit(status);
            }

            exec_search(command, env);
        }

        pids.push(pid);
        close_if_open(previous_read);
        if let Some((read_fd, write_fd)) = pipe {
            close_if_open(write_fd);
            previous_read = read_fd;
        } else {
            previous_read = -1;
        }
    }

    close_if_open(previous_read);

    for pid in pids {
        let mut status = 0;
        let _ = process::waitpid(pid, &mut status, 0);
    }

    Ok(())
}

fn apply_redirections<'a>(command: &ParsedCommand<'a>) -> Result<(), RedirectionError<'a>> {
    if let Some(path) = command.stdin {
        let fd = fs::open(path, fs::O_RDONLY, 0).map_err(|error| RedirectionError { path, error })?;
        io::dup2(fd, bindings::STDIN_FILENO as i32).map_err(|_| RedirectionError {
            path,
            error: fs::errno(),
        })?;
        close_if_open(fd);
    }

    if let Some(stdout) = command.stdout {
        let (path, flags) = match stdout {
            Redirect::Truncate(path) => (path, fs::O_CREAT | fs::O_WRONLY | fs::O_TRUNC),
            Redirect::Append(path) => (path, fs::O_CREAT | fs::O_WRONLY | fs::O_APPEND),
        };

        let fd = fs::open(path, flags, 0).map_err(|error| RedirectionError { path, error })?;
        io::dup2(fd, bindings::STDOUT_FILENO as i32).map_err(|_| RedirectionError {
            path,
            error: fs::errno(),
        })?;
        close_if_open(fd);
    }

    Ok(())
}

fn exec_search(command: &ParsedCommand, env: &ShellEnv) -> ! {
    let Some(name) = command.command() else {
        process::exit(127);
    };

    let env_entries = env.as_strs();
    if name.contains('/') {
        if executable_exists(name) {
            process::execve_with_env(name, &command.args, &env_entries);
        }
    } else {
        let path = env.get("PATH").unwrap_or("/bin");
        for directory in path.split(':') {
            let directory = if directory.is_empty() { "." } else { directory };
            let path = join_path(directory, name);
            if executable_exists(path.as_str()) {
                process::execve_with_env(path.as_str(), &command.args, &env_entries);
            }

            let path = format!("{}.elf", path);
            if executable_exists(path.as_str()) {
                process::execve_with_env(path.as_str(), &command.args, &env_entries);
            }
        }
    }

    process::exit(127);
}

fn executable_exists(path: &str) -> bool {
    fs::stat(path).map(|stat| !stat.is_dir()).unwrap_or(false)
}

fn close_if_open(fd: i32) {
    if fd >= 0 {
        let _ = io::close(fd);
    }
}

fn print_help() {
    println!("Builtins:");
    println!("  cd [dir]      change directory");
    println!("  pwd           print current directory");
    println!("  ls [-a] [path...] list files");
    println!("  cat <file...> print files");
    println!("  touch <file>  create file if missing");
    println!("  cp <src> <dst> copy file");
    println!("  mv <src> <dst> move file");
    println!("  mkdir <dir>   create directory");
    println!("  rmdir <dir>   remove empty directory");
    println!("  rm <file>     remove file");
    println!("  stat <path>   show file metadata");
    println!("  echo [text]   print text");
    println!("  env           print environment");
    println!("  export A=B    set environment variable");
    println!("  clear         clear screen");
    println!("  memory winsize devtest net dhcp ping dns reboot shutdown exit");
}

fn print_env(env: &ShellEnv) {
    for entry in env.entries.iter() {
        println!("{}", entry);
    }
}

fn export_env(env: &mut ShellEnv, args: &[&str]) {
    if args.is_empty() {
        print_env(env);
        return;
    }

    for assignment in args {
        if let Err(error) = env.set_assignment(assignment) {
            println!("{}", error);
        }
    }
}

fn valid_env_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    for (index, byte) in name.bytes().enumerate() {
        let valid = byte == b'_' || byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit());
        if !valid {
            return false;
        }
    }

    true
}

fn join_path(directory: &str, name: &str) -> String {
    if directory == "/" {
        format!("/{}", name)
    } else {
        format!("{}/{}", directory, name)
    }
}

fn print_pwd() {
    match fs::getcwd() {
        Ok(cwd) => println!("{}", cwd),
        Err(error) => println!("pwd: {}", errno_name(error)),
    }
}

fn change_directory(path: &str) {
    if let Err(error) = fs::chdir(path) {
        print_fs_error("cd", path, error);
    }
}

fn list_paths(args: &[&str]) {
    let show_all = args.iter().any(|arg| *arg == "-a");
    let paths: Vec<&str> = args
        .iter()
        .copied()
        .filter(|arg| !arg.starts_with('-'))
        .collect();

    if paths.is_empty() {
        list_path(".", show_all);
        return;
    }

    let multiple = paths.len() > 1;
    for (index, path) in paths.iter().enumerate() {
        if multiple {
            if index > 0 {
                println!();
            }
            println!("{}:", path);
        }
        list_path(path, show_all);
    }
}

fn list_path(path: &str, show_all: bool) {
    match fs::stat(path) {
        Ok(stat) if stat.is_dir() => match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    if !show_all && entry.name.starts_with('.') {
                        continue;
                    }

                    if entry.file_type == fs::DT_DIR {
                        print!("{}/  ", entry.name);
                    } else {
                        print!("{}  ", entry.name);
                    }
                }
                println!();
            }
            Err(error) => print_fs_error("ls", path, error),
        },
        Ok(_) => println!("{}", path),
        Err(error) => print_fs_error("ls", path, error),
    }
}

fn cat_files(paths: &[&str]) {
    if paths.is_empty() {
        cat_fd(bindings::STDIN_FILENO as i32, "stdin");
        return;
    }

    for path in paths {
        cat_file(path);
    }
}

fn cat_file(path: &str) {
    let fd = match fs::open(path, fs::O_RDONLY, 0) {
        Ok(fd) => fd,
        Err(error) => {
            print_fs_error("cat", path, error);
            return;
        }
    };

    cat_fd(fd, path);
    let _ = fs::close(fd);
}

fn cat_fd(fd: i32, label: &str) {
    let mut buffer = [0_u8; 512];
    loop {
        let read = unsafe {
            bindings::read(
                fd,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
            )
        };

        if read < 0 {
            print_fs_error("cat", label, fs::errno());
            break;
        }

        if read == 0 {
            break;
        }

        write_stdout(&buffer[..read as usize]);
    }
}

fn touch_files(paths: &[&str]) {
    if paths.is_empty() {
        println!("Usage: touch <file>...");
        return;
    }

    for path in paths {
        match fs::open(path, fs::O_CREAT | fs::O_RDWR, 0) {
            Ok(fd) => {
                let _ = fs::close(fd);
            }
            Err(error) => print_fs_error("touch", path, error),
        }
    }
}

fn copy_command(args: &[&str]) {
    if args.len() != 2 {
        println!("Usage: cp <src> <dst>");
        return;
    }

    if let Err(error) = copy_file(args[0], args[1]) {
        print_fs_error("cp", args[1], error);
    }
}

fn move_command(args: &[&str]) {
    if args.len() != 2 {
        println!("Usage: mv <src> <dst>");
        return;
    }

    match copy_file(args[0], args[1]) {
        Ok(()) => {
            if let Err(error) = fs::unlink(args[0]) {
                print_fs_error("mv", args[0], error);
            }
        }
        Err(error) => print_fs_error("mv", args[1], error),
    }
}

fn copy_file(src: &str, dst: &str) -> Result<(), i32> {
    let metadata = fs::stat(src)?;
    if metadata.is_dir() {
        return Err(bindings::EISDIR as i32);
    }

    let src_fd = fs::open(src, fs::O_RDONLY, 0)?;
    let _ = fs::unlink(dst);
    let dst_fd = match fs::open(dst, fs::O_CREAT | fs::O_WRONLY, 0) {
        Ok(fd) => fd,
        Err(error) => {
            let _ = fs::close(src_fd);
            return Err(error);
        }
    };

    let mut buffer = [0_u8; 512];
    loop {
        let read = match io::read(src_fd, &mut buffer) {
            Ok(read) => read,
            Err(_) => {
                let error = fs::errno();
                let _ = fs::close(src_fd);
                let _ = fs::close(dst_fd);
                return Err(error);
            }
        };

        if read == 0 {
            break;
        }

        if write_fd(dst_fd, &buffer[..read]).is_err() {
            let error = fs::errno();
            let _ = fs::close(src_fd);
            let _ = fs::close(dst_fd);
            return Err(error);
        }
    }

    fs::close(src_fd)?;
    fs::close(dst_fd)?;
    Ok(())
}

fn make_directories(paths: &[&str]) {
    if paths.is_empty() {
        println!("Usage: mkdir <dir>...");
        return;
    }

    for path in paths {
        if let Err(error) = fs::mkdir(path, 0) {
            print_fs_error("mkdir", path, error);
        }
    }
}

fn remove_directories(paths: &[&str]) {
    if paths.is_empty() {
        println!("Usage: rmdir <dir>...");
        return;
    }

    for path in paths {
        if let Err(error) = fs::rmdir(path) {
            print_fs_error("rmdir", path, error);
        }
    }
}

fn remove_files(paths: &[&str]) {
    if paths.is_empty() {
        println!("Usage: rm <file>...");
        return;
    }

    for path in paths {
        if let Err(error) = fs::unlink(path) {
            print_fs_error("rm", path, error);
        }
    }
}

fn stat_paths(paths: &[&str]) {
    if paths.is_empty() {
        println!("Usage: stat <path>...");
        return;
    }

    for path in paths {
        match fs::lstat(path) {
            Ok(stat) => {
                let kind = if stat.is_dir() {
                    "directory"
                } else if stat.is_file() {
                    "file"
                } else {
                    "unknown"
                };
                println!(
                    "{}: type={} size={} mode={:o} uid={} gid={}",
                    path, kind, stat.size, stat.mode, stat.uid, stat.gid
                );
            }
            Err(error) => print_fs_error("stat", path, error),
        }
    }
}

fn write_stdout(data: &[u8]) {
    let _ = write_fd(bindings::STDOUT_FILENO as i32, data);
}

fn write_fd(fd: i32, mut data: &[u8]) -> Result<(), ()> {
    while !data.is_empty() {
        match io::write(fd, data) {
            Ok(0) | Err(_) => break,
            Ok(written) => data = &data[written..],
        }
    }

    if data.is_empty() { Ok(()) } else { Err(()) }
}

fn print_fs_error(command: &str, path: &str, error: i32) {
    println!("{}: {}: {}", command, path, errno_name(error));
}

fn print_redirection_error(error: &RedirectionError) {
    let message = format!("shell: {}: {}\n", error.path, errno_name(error.error));
    let _ = write_fd(bindings::STDERR_FILENO as i32, message.as_bytes());
}

fn errno_name(error: i32) -> &'static str {
    match error as u32 {
        bindings::ENOENT => "No such file or directory",
        bindings::EEXIST => "File exists",
        bindings::ENOTDIR => "Not a directory",
        bindings::EISDIR => "Is a directory",
        bindings::ENOTEMPTY => "Directory not empty",
        bindings::EACCES => "Permission denied",
        bindings::EBADF => "Bad file descriptor",
        bindings::EINVAL => "Invalid argument",
        bindings::ENOMEM => "Out of memory",
        bindings::ENOSYS => "Function not implemented",
        bindings::ENOTSUP => "Operation not supported",
        _ => "I/O error",
    }
}

fn print_winsize() {
    let mut size = bindings::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let result = unsafe {
        bindings::ioctl(
            bindings::STDOUT_FILENO as i32,
            bindings::TIOCGWINSZ as core::ffi::c_ulong,
            &mut size as *mut bindings::winsize as core::ffi::c_ulong,
        )
    };

    if result < 0 {
        println!("winsize: ioctl failed");
        return;
    }

    println!("screen: {} cols x {} rows", size.ws_col, size.ws_row);
}

fn devtest() {
    test_device_write(b"/dev/serial\0", b"/dev/serial write ok\n", "/dev/serial");
    test_device_write(b"/dev/screen\0", b"/dev/screen write ok\n", "/dev/screen");
    test_device_write(b"/dev/null\0", b"/dev/null write ok\n", "/dev/null");
}

fn test_device_write(path: &'static [u8], message: &'static [u8], label: &str) {
    let fd = unsafe { bindings::open(path.as_ptr() as *const i8, bindings::O_WRONLY as i32, 0) };

    if fd < 0 {
        println!("{}: open failed", label);
        return;
    }

    let written = unsafe { bindings::write(fd, message.as_ptr() as *const c_void, message.len()) };
    unsafe {
        bindings::close(fd);
    }

    if written == message.len() as isize {
        println!("{}: ok", label);
    } else {
        println!("{}: write failed", label);
    }
}

fn print_network_info() {
    let mut info = bindings::network_info {
        present: 0,
        dhcp_state: 0,
        mac: [0; 6],
        _padding: [0; 2],
        ipv4: [0; 4],
        subnet_mask: [0; 4],
        router: [0; 4],
        dns: [0; 4],
        packets_rx: 0,
        packets_tx: 0,
        arp_entries: 0,
        ping_tx: 0,
        ping_rx: 0,
        dns_tx: 0,
        dns_rx: 0,
    };

    if unsafe { bindings::network_info(&mut info) } < 0 || info.present == 0 {
        println!("Network: no RTL8139 device");
        return;
    }

    println!(
        "Network: rtl8139 mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} rx={} tx={} dhcp={} arp={} ping={}/{} dns={}/{}",
        info.mac[0],
        info.mac[1],
        info.mac[2],
        info.mac[3],
        info.mac[4],
        info.mac[5],
        info.packets_rx,
        info.packets_tx,
        dhcp_state_name(info.dhcp_state),
        info.arp_entries,
        info.ping_rx,
        info.ping_tx,
        info.dns_rx,
        info.dns_tx
    );

    if info.ipv4 != [0; 4] {
        println!(
            "IPv4: {}.{}.{}.{}/{}.{}.{}.{} gateway={}.{}.{}.{} dns={}.{}.{}.{}",
            info.ipv4[0],
            info.ipv4[1],
            info.ipv4[2],
            info.ipv4[3],
            info.subnet_mask[0],
            info.subnet_mask[1],
            info.subnet_mask[2],
            info.subnet_mask[3],
            info.router[0],
            info.router[1],
            info.router[2],
            info.router[3],
            info.dns[0],
            info.dns[1],
            info.dns[2],
            info.dns[3]
        );
    }
}

fn send_dhcp_discover() {
    let res = unsafe { bindings::network_dhcp_discover() };
    if res < 0 {
        println!("DHCP discover failed");
        return;
    }

    println!("DHCP discover sent");
    print_network_info();
}

fn ping_gateway() {
    let res = unsafe { bindings::network_ping_gateway() };
    if res < 0 {
        println!("Ping failed: run dhcp first");
    }
}

fn ping_ipv4(ip: u32) {
    let socket = unsafe {
        bindings::socket(
            bindings::AF_INET as i32,
            bindings::SOCK_RAW as i32,
            bindings::IPPROTO_ICMP as i32,
        )
    };
    if socket < 0 {
        println!("Ping failed: socket");
        return;
    }

    let addr = bindings::sockaddr_in {
        sin_family: bindings::AF_INET as u16,
        sin_port: 0,
        sin_addr: bindings::in_addr { s_addr: ip },
        sin_zero: [0; 8],
    };

    let res = unsafe {
        bindings::sendto(
            socket,
            core::ptr::null::<core::ffi::c_void>(),
            64,
            0,
            &addr as *const bindings::sockaddr_in as *const bindings::sockaddr,
            core::mem::size_of::<bindings::sockaddr_in>() as u32,
        )
    };

    if res < 0 {
        unsafe {
            bindings::close(socket);
        }
        println!("Ping failed: run dhcp first");
        return;
    }

    if !recv_ping_reply(socket) {
        println!("Ping failed: timeout");
    }

    unsafe {
        bindings::close(socket);
    }
}

fn recv_ping_reply(socket: i32) -> bool {
    let mut packet = [0u8; 128];
    let mut src = bindings::sockaddr_in {
        sin_family: 0,
        sin_port: 0,
        sin_addr: bindings::in_addr { s_addr: 0 },
        sin_zero: [0; 8],
    };
    let mut src_len = core::mem::size_of::<bindings::sockaddr_in>() as bindings::socklen_t;

    let res = unsafe {
        bindings::recvfrom_wait(
            socket,
            packet.as_mut_ptr() as *mut core::ffi::c_void,
            packet.len(),
            0,
            &mut src as *mut bindings::sockaddr_in as *mut bindings::sockaddr,
            &mut src_len,
            5_000_000,
        )
    };

    res >= 0
}

fn ping_name(name: &str) {
    let res = unsafe { bindings::network_ping_name(name.as_ptr() as *const i8) };
    if res < 0 {
        println!("Ping failed: run dhcp first");
    }
}

fn send_dns_query(name: &str) {
    let res = unsafe { bindings::network_dns_query(name.as_ptr() as *const i8) };
    if res < 0 {
        println!("DNS query failed: run dhcp first");
        return;
    }

    println!("DNS query sent");
    print_network_info();
}

fn parse_ipv4(input: &str) -> Option<u32> {
    let mut octets = [0u8; 4];
    let mut count = 0;

    for part in input.split('.') {
        if count >= octets.len() {
            return None;
        }

        octets[count] = parse_ipv4_octet(part)?;
        count += 1;
    }

    if count != octets.len() {
        return None;
    }

    Some(
        ((octets[0] as u32) << 24)
            | ((octets[1] as u32) << 16)
            | ((octets[2] as u32) << 8)
            | octets[3] as u32,
    )
}

fn parse_ipv4_octet(input: &str) -> Option<u8> {
    if input.is_empty() {
        return None;
    }

    let mut value = 0u16;
    for byte in input.bytes() {
        if byte < b'0' || byte > b'9' {
            return None;
        }

        value = value * 10 + (byte - b'0') as u16;
        if value > 255 {
            return None;
        }
    }

    Some(value as u8)
}

fn dhcp_state_name(state: u32) -> &'static str {
    match state {
        0 => "init",
        1 => "selecting",
        2 => "requesting",
        3 => "bound",
        _ => "unknown",
    }
}
