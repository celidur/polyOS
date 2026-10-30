use crate::{ast::*, env::ShellEnv, jobs::JobManager};

fn expand_tilde_argv(env: &ShellEnv, pl: &mut Pipeline) {
    let home = env.get("HOME").map(|s| s.to_string());
    for cmd in &mut pl.cmds {
        for tok in &mut cmd.argv {
            *tok = crate::env::expand_tilde(tok, home.as_deref());
        }
    }
}

fn expand_aliases(env: &ShellEnv, pl: &mut Pipeline) {
    // cap at 8 passes to avoid infinite alias loops
    const LIMIT: usize = 8;

    for cmd in &mut pl.cmds {
        for _ in 0..LIMIT {
            let Some(head) = cmd.argv.first().cloned() else { break; };
            let Some(a) = env.get_alias(&head).map(|s| s.to_string()) else { break; };

            // expand alias value into words, then tack on the remaining args
            let mut expanded = match shell_words::split(&a) {
                Ok(v) => v,
                Err(_) => break,
            };
            expanded.extend(cmd.argv.iter().skip(1).cloned());
            cmd.argv = expanded;
        }
    }
}

#[cfg(unix)]
mod imp {
    use super::*;
    use crate::builtins::{is_builtin, try_run_builtin, BuiltinResult};
    use anyhow::{bail, Context};
    use nix::{
        fcntl::{open, OFlag},
        libc,
        sys::{
            signal::{signal, SigHandler, Signal},
            stat::Mode,
            wait::{waitpid, WaitPidFlag, WaitStatus},
        },
        unistd::{dup2, execvp, fork, getpgrp, pipe2, setpgid, tcsetpgrp, ForkResult, Pid},
    };
    use std::os::fd::BorrowedFd;
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, OwnedFd};

    pub struct Shell {
        pub env: ShellEnv,
        pub jobs: JobManager,
        pub last_exit: i32,
    }

    impl Shell {
        pub fn new() -> anyhow::Result<Self> {
            let mut jobs = JobManager::new();
            jobs.init_tty();
            Ok(Self { env: ShellEnv::new()?, jobs, last_exit: 0 })
        }

        pub fn run_pipeline(&mut self, mut pl: Pipeline, cmdline: String) -> anyhow::Result<i32> {
            expand_tilde_argv(&self.env, &mut pl);
            // Aliases must expand before the builtin check so that `alias cd2=cd`
            // causes cd2 to be recognised as the builtin cd.
            expand_aliases(&self.env, &mut pl);

            // builtins only for a single foreground command — pipes and bg go through fork
            if pl.cmds.len() == 1 && !pl.background {
                let command = &pl.cmds[0];
                let argv = &command.argv;
                if argv.first().is_some_and(|name| is_builtin(name)) {
                    // Builtins run in the parent so cd/export retain their effects.
                    // Restore its descriptors on success and every error path.
                    let _redirects = BuiltinRedirects::capture(command)?;
                    let result = (|| -> anyhow::Result<BuiltinResult> {
                        apply_redirects(command)?;
                        try_run_builtin(&mut self.env, &mut self.jobs, argv)?
                            .context("recognized builtin was not dispatched")
                    })();
                    return Ok(match result {
                        Ok(BuiltinResult::Continue) => 0,
                        Ok(BuiltinResult::Exit(code)) => code,
                        Err(error) => {
                            // Report while stderr still has its redirected destination.
                            eprintln!("orbis: {error}");
                            1
                        }
                    });
                }
            }

            // Hold children until the parent has assigned the whole process group
            // and (for foreground jobs) transferred terminal ownership.
            let (launch_read, launch_write) = pipe2(OFlag::O_CLOEXEC)?;
            let mut pids = Vec::new();
            let mut pgid: Option<Pid> = None;
            let mut prev_read: Option<OwnedFd> = None;

            for (idx, cmd) in pl.cmds.iter().enumerate() {
                let is_last = idx == pl.cmds.len() - 1;
                let (r, w): (Option<OwnedFd>, Option<OwnedFd>) = if !is_last {
                    // O_CLOEXEC: both ends close automatically in the child after
                    // execvp replaces the process image. dup2 clears CLOEXEC on the
                    // target fd (POSIX), so stdin/stdout remapping survives exec.
                    let (r, w) = pipe2(OFlag::O_CLOEXEC)?;
                    (Some(r), Some(w))
                } else {
                    (None, None)
                };

                match unsafe { fork()? } {
                    ForkResult::Child => {
                        // Every fallible setup step must stay inside this boundary.
                        // Returning to the caller here would start a second shell.
                        let setup = (|| -> anyhow::Result<()> {
                        // put the child in its own process group right away
                        let mypid = nix::unistd::getpid();
                        let target_pgid = pgid.unwrap_or(mypid);
                        setpgid(mypid, target_pgid)?;
                        for sig in [Signal::SIGINT, Signal::SIGQUIT, Signal::SIGTSTP, Signal::SIGTTIN, Signal::SIGTTOU] {
                            unsafe { signal(sig, SigHandler::SigDfl)?; }
                        }
                        drop(launch_write);
                        let mut release = [0u8; 1];
                        loop {
                            match nix::unistd::read(launch_read.as_raw_fd(), &mut release) {
                                Err(nix::errno::Errno::EINTR) => continue,
                                result => { result?; break; }
                            }
                        }
                        drop(launch_read);

                        // wire up stdin/stdout to the pipe ends
                        if let Some(fd) = prev_read.as_ref() {
                            dup2(fd.as_raw_fd(), libc::STDIN_FILENO)?;
                        }
                        if let Some(fd) = w.as_ref() {
                            dup2(fd.as_raw_fd(), libc::STDOUT_FILENO)?;
                        }

                        // Drop all pipe OwnedFds explicitly: O_CLOEXEC closes them
                        // when execvp succeeds; explicit drops close them when execvp
                        // fails (so the error path doesn't leak fds either).
                        drop(prev_read.take());
                        drop(r);
                        drop(w);

                        apply_redirects(cmd)?;

                        // replace the process image
                        if cmd.argv.is_empty() {
                            bail!("empty command");
                        }
                        let c_argv: Vec<CString> = cmd
                            .argv
                            .iter()
                            .map(|s| CString::new(s.as_str()).context("NUL dans argv"))
                            .collect::<anyhow::Result<_>>()?;
                        let error = execvp(&c_argv[0], &c_argv).unwrap_err();
                        #[cfg(target_vendor = "polyos")]
                        if error == nix::errno::Errno::ENOENT && matches!(cmd.argv[0].as_str(),
                            "orbisbox" | "ls" | "cat" | "cp" | "mv" | "rm" | "mkdir" | "rmdir" | "touch" | "ln" |
                            "chmod" | "stat" | "du" | "df" | "find" | "head" | "tail" | "wc" | "tee" |
                            "grep" | "sort" | "uniq" | "cut" | "tr" | "seq" | "diff" | "xargs" |
                            "echo" | "pwd" | "which" | "whoami" | "uname" | "date" | "sleep" | "yes" |
                            "env" | "id" | "ps" | "kill" | "basename" | "dirname" | "realpath") {
                            let utility = CString::new("/bin/orbisbox.elf").unwrap();
                            let mut args = vec![utility.clone()];
                            if cmd.argv[0] == "orbisbox" { args.extend(c_argv.into_iter().skip(1)); }
                            else { args.extend(c_argv); }
                            let _ = execvp(&utility, &args);
                        }
                        eprintln!("orbis: {}: {}", cmd.argv[0], error);
                        unsafe { libc::_exit(127); }
                        })();
                        if let Err(error) = setup {
                            eprintln!("orbis: child setup: {error}");
                        }
                        unsafe { libc::_exit(126); }
                    }
                    ForkResult::Parent { child } => {
                        // parent also sets pgid to avoid a race with the child
                        if pgid.is_none() {
                            pgid = Some(child);
                        }
                        let _ = setpgid(child, pgid.unwrap());

                        prev_read = r;
                        // w drops here — write end of pipe is gone from the parent
                        pids.push(child);
                    }
                }
            }

            if pl.background {
                drop(launch_read); drop(launch_write);
                let id = self.jobs.add(pgid.context("pgid absent")?, pids, cmdline);
                println!("[{id}] {}", pgid.unwrap());
                Ok(0)
            } else {
                // give the terminal to the child's process group so Ctrl+C targets
                // the child and not the shell
                let shell_pgid = getpgrp();
                if let Some(pg) = pgid {
                    let tty = unsafe { BorrowedFd::borrow_raw(libc::STDIN_FILENO) };
                    let _ = tcsetpgrp(tty, pg);
                }

                drop(launch_read); drop(launch_write);

                // ignore SIGINT / SIGTSTP in the shell while the child is running
                let old_sigint = unsafe { signal(Signal::SIGINT, SigHandler::SigIgn) };
                let old_sigtstp = unsafe { signal(Signal::SIGTSTP, SigHandler::SigIgn) };

                // wait on all pids; WUNTRACED lets waitpid return when Ctrl+Z stops the job
                let mut last_code = 0i32;
                let mut was_stopped = false;
                for pid in &pids {
                    loop {
                        match waitpid(*pid, Some(WaitPidFlag::WUNTRACED)) {
                            Ok(WaitStatus::Exited(_, code)) => { last_code = code; break; }
                            Ok(WaitStatus::Signaled(_, sig, _)) => { last_code = 128 + sig as i32; break; }
                            Ok(WaitStatus::Stopped(_, _)) => { was_stopped = true; break; }
                            Ok(_) => continue,
                            Err(_) => break, // ECHILD: already reaped
                        }
                    }
                }

                // restore terminal control to the shell
                let tty = unsafe { BorrowedFd::borrow_raw(libc::STDIN_FILENO) };
                let _ = tcsetpgrp(tty, shell_pgid);

                // restore signal handlers
                if let Ok(old) = old_sigint {
                    unsafe { let _ = signal(Signal::SIGINT, old); }
                }
                if let Ok(old) = old_sigtstp {
                    unsafe { let _ = signal(Signal::SIGTSTP, old); }
                }

                if was_stopped {
                    let id = self.jobs.add(pgid.context("pgid absent")?, pids, cmdline.clone());
                    self.jobs.mark_stopped(id);
                    eprintln!("\n[{id}] Stopped   {cmdline}");
                    last_code = 128 + libc::SIGTSTP;
                }

                Ok(last_code)
            }
        }
    }

    struct BuiltinRedirects {
        saved: Vec<(i32, Option<(OwnedFd, i32)>)>,
    }
    impl BuiltinRedirects {
        fn capture(cmd: &Command) -> anyhow::Result<Self> {
            use nix::fcntl::{fcntl, FcntlArg};
            use std::{io::Write, os::fd::FromRawFd};
            std::io::stdout().flush()?;
            std::io::stderr().flush()?;
            let mut restore = Self { saved: Vec::new() };
            for redirect in &cmd.redirects {
                let target = match redirect.fd {
                    Fd::Stdin => libc::STDIN_FILENO,
                    Fd::Stdout => libc::STDOUT_FILENO,
                    Fd::Stderr => libc::STDERR_FILENO,
                };
                if restore.saved.iter().any(|(fd, _)| *fd == target) { continue; }
                let saved = match fcntl(target, FcntlArg::F_GETFD) {
                    Ok(flags) => {
                        let copy = fcntl(target, FcntlArg::F_DUPFD_CLOEXEC(3))?;
                        // fcntl returned a new descriptor owned by this guard.
                        Some((unsafe { OwnedFd::from_raw_fd(copy) }, flags))
                    }
                    Err(nix::errno::Errno::EBADF) => None,
                    Err(error) => return Err(error.into()),
                };
                restore.saved.push((target, saved));
            }
            Ok(restore)
        }
    }
    impl Drop for BuiltinRedirects {
        fn drop(&mut self) {
            use nix::fcntl::{fcntl, FcntlArg, FdFlag};
            use std::io::Write;
            // Buffered builtin output belongs to the redirected destination.
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
            for (target, saved) in &self.saved {
                if let Some((copy, flags)) = saved {
                    let _ = dup2(copy.as_raw_fd(), *target);
                    let _ = fcntl(*target, FcntlArg::F_SETFD(FdFlag::from_bits_truncate(*flags)));
                } else {
                    let _ = nix::unistd::close(*target);
                }
            }
        }
    }

    fn apply_redirects(cmd: &Command) -> anyhow::Result<()> {
        use anyhow::bail;
        use std::os::fd::AsRawFd;

        for r in &cmd.redirects {
            let path = match &r.target {
                RedirectTarget::Path(p) => p.as_str(),
            };
            match (r.fd, r.mode) {
                (Fd::Stdin, RedirectMode::Read) => {
                    let fd = open(path, OFlag::O_RDONLY, Mode::empty())?;
                    dup2(fd.as_raw_fd(), libc::STDIN_FILENO)?;
                }
                (Fd::Stdout, RedirectMode::WriteTrunc) => {
                    let fd = open(
                        path,
                        OFlag::O_CREAT | OFlag::O_WRONLY | OFlag::O_TRUNC,
                        Mode::from_bits_truncate(0o644),
                    )?;
                    dup2(fd.as_raw_fd(), libc::STDOUT_FILENO)?;
                }
                (Fd::Stdout, RedirectMode::WriteAppend) => {
                    let fd = open(
                        path,
                        OFlag::O_CREAT | OFlag::O_WRONLY | OFlag::O_APPEND,
                        Mode::from_bits_truncate(0o644),
                    )?;
                    dup2(fd.as_raw_fd(), libc::STDOUT_FILENO)?;
                }
                (Fd::Stderr, RedirectMode::WriteTrunc) => {
                    let fd = open(
                        path,
                        OFlag::O_CREAT | OFlag::O_WRONLY | OFlag::O_TRUNC,
                        Mode::from_bits_truncate(0o644),
                    )?;
                    dup2(fd.as_raw_fd(), libc::STDERR_FILENO)?;
                }
                // other fd combos (2>&1 etc.) not worth implementing right now
                _ => bail!("unsupported redirect"),
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
pub use imp::Shell;

#[cfg(not(unix))]
pub struct Shell {
    pub env: ShellEnv,
    pub jobs: JobManager,
    pub last_exit: i32,
}

#[cfg(not(unix))]
impl Shell {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self { env: ShellEnv::new()?, jobs: JobManager::new(), last_exit: 0 })
    }

    pub fn run_pipeline(&mut self, mut pl: Pipeline, cmdline: String) -> anyhow::Result<i32> {
        use crate::builtins::{is_builtin, try_run_builtin, BuiltinResult};
        use anyhow::{bail, Context};
        use std::{
            fs::OpenOptions,
            process::{Command, Stdio},
        };

        expand_tilde_argv(&self.env, &mut pl);
        expand_aliases(&self.env, &mut pl);

        // builtins only for single foreground commands
        if pl.cmds.len() == 1 && !pl.background {
            let argv = &pl.cmds[0].argv;
            if let Some(r) = try_run_builtin(&mut self.env, &mut self.jobs, argv)? {
                return Ok(match r {
                    BuiltinResult::Continue => 0,
                    BuiltinResult::Exit(code) => code,
                });
            }
        }

        let mut children: Vec<std::process::Child> = Vec::new();
        let mut prev_stdout: Option<std::process::ChildStdout> = None;

        for (idx, cmd) in pl.cmds.iter().enumerate() {
            let is_last = idx == pl.cmds.len() - 1;
            if cmd.argv.is_empty() {
                bail!("empty command");
            }

            let mut c = Command::new(&cmd.argv[0]);
            c.args(&cmd.argv[1..]);
            c.current_dir(&self.env.cwd);
            c.envs(self.env.vars.iter().map(|(k, v)| (k, v)));

            if let Some(out) = prev_stdout.take() {
                c.stdin(Stdio::from(out));
            } else {
                c.stdin(Stdio::inherit());
            }

            c.stderr(Stdio::inherit());
            if !is_last {
                c.stdout(Stdio::piped());
            } else {
                c.stdout(Stdio::inherit());
            }

            for r in &cmd.redirects {
                let path = match &r.target {
                    RedirectTarget::Path(p) => p,
                };

                match (r.fd, r.mode) {
                    (Fd::Stdin, RedirectMode::Read) => {
                        let f = OpenOptions::new()
                            .read(true)
                            .open(path)
                            .with_context(|| format!("open < {path}"))?;
                        c.stdin(Stdio::from(f));
                    }
                    (Fd::Stdout, RedirectMode::WriteTrunc) => {
                        let f = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .truncate(true)
                            .open(path)
                            .with_context(|| format!("open > {path}"))?;
                        c.stdout(Stdio::from(f));
                    }
                    (Fd::Stdout, RedirectMode::WriteAppend) => {
                        let f = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .append(true)
                            .open(path)
                            .with_context(|| format!("open >> {path}"))?;
                        c.stdout(Stdio::from(f));
                    }
                    (Fd::Stderr, RedirectMode::WriteTrunc) => {
                        let f = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .truncate(true)
                            .open(path)
                            .with_context(|| format!("open 2> {path}"))?;
                        c.stderr(Stdio::from(f));
                    }
                    _ => bail!("unsupported redirect"),
                }
            }

            let mut child = c.spawn().with_context(|| format!("spawn {}", cmd.argv[0]))?;

            if !is_last {
                prev_stdout = child.stdout.take();
            }

            children.push(child);
        }

        if pl.background {
            let id = self.jobs.add(children, cmdline);
            println!("[{id}] Running");
            Ok(0)
        } else {
            let mut last_code = 0;
            for (i, mut ch) in children.into_iter().enumerate() {
                let st = ch.wait()?;
                if i == pl.cmds.len().saturating_sub(1) {
                    last_code = st.code().unwrap_or(1);
                }
            }
            Ok(last_code)
        }
    }

    pub fn run_line(&mut self, line: &str) -> anyhow::Result<i32> {
        let result = run_line_impl(self, line);
        if let Ok(code) = result {
            self.last_exit = code;
        }
        result
    }
}

/// Shared run_line logic — platform-specific dispatch happens below.
fn run_line_impl(shell: &mut Shell, line: &str) -> anyhow::Result<i32> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(0);
    }

    let (background, core_line) = {
        let t = line.trim_end();
        if t.ends_with('&') && !t.ends_with("&&") {
            (true, t.trim_end_matches('&').trim().to_string())
        } else {
            (false, t.to_string())
        }
    };

    #[cfg(windows)]
    if should_delegate_to_cmd(&core_line) {
        return shell.run_via_cmd_exe(&core_line, background); // let cmd.exe handle it
    }

    #[cfg(unix)]
    if should_delegate_to_posix_shell(&core_line) {
        return shell.run_via_bash_or_sh(&core_line, background); // bash handles what we don't
    }

    // expand $VAR / ${VAR} / $? / $$ — after delegation so bash still gets raw text for &&/||
    let core_line = crate::env::expand_vars(&core_line, &shell.env, shell.last_exit);
    // expand globs (respects single/double quote context in the raw line)
    let core_line = crate::env::expand_globs_in_line(&core_line, &shell.env.cwd, shell.env.get("HOME"));

    match crate::parse_line(&core_line) {
        Ok(Some(pl)) => {
            let mut pl = pl;
            pl.background = background;

            match shell.run_pipeline(pl, core_line.clone()) {
                Ok(code) => Ok(code),

                #[cfg(windows)]
                Err(e) => {
                    if looks_like_not_found(&e) {
                        return shell.run_via_cmd_exe(&core_line, background);
                    }
                    Err(e)
                }

                #[cfg(unix)]
                Err(e) => {
                    if looks_like_not_found(&e) {
                        return shell.run_via_bash_or_sh(&core_line, background);
                    }
                    Err(e)
                }

                #[cfg(not(any(windows, unix)))]
                Err(e) => Err(e),
            }
        }

        Ok(None) => Ok(0),

        Err(_) => {
            #[cfg(windows)]
            return shell.run_via_cmd_exe(&core_line, background);

            #[cfg(unix)]
            return shell.run_via_bash_or_sh(&core_line, background);

            #[cfg(not(any(windows, unix)))]
            anyhow::bail!("Orbis: unsupported platform");
        }
    }
}

#[cfg(windows)]
fn should_delegate_to_cmd(line: &str) -> bool {
    // anything with &&, ||, or parens is beyond what the MVP parser handles
    if line.contains("&&") || line.contains("||") || line.contains('(') || line.contains(')') {
        return true;
    }
    // & in cmd is a command separator — if it's not at the end (background), let cmd handle it
    if let Some(pos) = line.find('&') {
        let last = line.trim_end().len().saturating_sub(1);
        if pos != last {
            return true;
        }
    }

    // cmd-internal commands that don't exist as standalone executables
    let head = line.split_whitespace().next().unwrap_or("").trim_matches('"').to_ascii_lowercase();
    matches!(
        head.as_str(),
        "dir" | "copy" | "del" | "erase" | "move" | "type" | "ren" | "rename" |
        "md" | "mkdir" | "rd" | "rmdir" | "cls" | "set" | "call" | "start" |
        "title" | "ver" | "vol" | "path" | "assoc" | "ftype" | "pushd" | "popd"
    )
}

#[cfg(unix)]
fn should_delegate_to_posix_shell(line: &str) -> bool {
    line.contains("&&")
        || line.contains("||")
        || line.contains(';')
        || line.contains("$(")
        || line.contains('`')
        || line.contains("{")
        || line.contains("}")
}

#[cfg(any(windows, unix))]
fn looks_like_not_found(e: &anyhow::Error) -> bool {
    if let Some(ioe) = e.downcast_ref::<std::io::Error>() {
        return ioe.kind() == std::io::ErrorKind::NotFound;
    }
    let msg = e.to_string().to_ascii_lowercase();
    msg.contains("not found") || msg.contains("no such file")
}

#[cfg(windows)]
impl Shell {
    fn run_via_cmd_exe(&mut self, line: &str, background: bool) -> anyhow::Result<i32> {
        use std::process::Command;

        let mut child = Command::new("cmd.exe")
            .args(["/C", line])
            .current_dir(&self.env.cwd)
            .envs(self.env.vars.iter().map(|(k, v)| (k, v)))
            .spawn()?;

        if background {
            let _id = self.jobs.add(vec![child], line.to_string());
            return Ok(0);
        }

        let st = child.wait()?;
        Ok(st.code().unwrap_or(1))
    }
}

#[cfg(unix)]
impl Shell {
    pub fn run_line(&mut self, line: &str) -> anyhow::Result<i32> {
        let result = run_line_impl(self, line);
        if let Ok(code) = result {
            self.last_exit = code;
        }
        result
    }

    fn run_via_bash_or_sh(&mut self, line: &str, background: bool) -> anyhow::Result<i32> {
        use nix::unistd::Pid;
        use std::process::Command;

        let mut cmd = Command::new("bash");
        cmd.args(["-c", line])
            .current_dir(&self.env.cwd)
            .envs(self.env.vars.iter().map(|(k, v)| (k, v)));

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => {
                let mut sh = Command::new("sh");
                sh.args(["-c", line])
                    .current_dir(&self.env.cwd)
                    .envs(self.env.vars.iter().map(|(k, v)| (k, v)));
                sh.spawn()?
            }
        };

        if background {
            let pid = Pid::from_raw(child.id() as i32);
            let _id = self.jobs.add(pid, vec![pid], line.to_string());
            return Ok(0);
        }

        let st = child.wait()?;
        Ok(st.code().unwrap_or(1))
    }
}
