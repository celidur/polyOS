//! Process groups, foreground-terminal ownership, and stopped-job lifetime.
use polyos_std::*;

const JOB_SIGNALS: [i32; 5] = [2, 3, 20, 21, 22];
const WUNTRACED: i32 = 2;
const WNOHANG: i32 = 1;

pub struct Launch {
    gate: (i32, i32),
    group: i32,
    pids: Vec<i32>,
}
impl Launch {
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self {
            gate: io::pipe().map_err(|_| "job pipe failed")?,
            group: 0,
            pids: Vec::new(),
        })
    }
    pub fn child(&self) {
        unsafe {
            if bindings::setpgid(0, self.group) < 0 {
                process::exit(126);
            }
            for signal in JOB_SIGNALS {
                bindings::signal(signal, None);
            }
        }
        let _ = io::close(self.gate.1);
        // EOF releases every child only after the parent has foregrounded the
        // complete group. No child can read the terminal before that handoff.
        if io::read(self.gate.0, &mut [0u8]).is_err() {
            process::exit(126);
        }
        let _ = io::close(self.gate.0);
    }
    pub fn add(&mut self, pid: i32) -> Result<(), &'static str> {
        if self.group == 0 {
            self.group = pid;
        }
        self.pids.push(pid);
        if unsafe { bindings::setpgid(pid, self.group) } < 0 {
            return Err("cannot create job process group");
        }
        Ok(())
    }
    pub fn abort(self) {
        for &pid in &self.pids {
            unsafe {
                bindings::kill(pid, 9);
            }
        }
        let _ = io::close(self.gate.0);
        let _ = io::close(self.gate.1);
        for pid in self.pids {
            let mut status = 0;
            process::waitpid(pid, &mut status, 0);
        }
    }
}

struct Member {
    pid: i32,
    stopped: bool,
}
struct Job {
    id: usize,
    group: i32,
    members: Vec<Member>,
    label: String,
    modes: Option<bindings::termios>,
    last_pid: i32,
    status: i32,
}
pub struct Jobs {
    shell: i32,
    interactive: bool,
    next_id: usize,
    stopped: Vec<Job>,
}
fn modes() -> Option<bindings::termios> {
    let mut value = unsafe { core::mem::zeroed() };
    (unsafe { bindings::tcgetattr(0, &mut value) } == 0).then_some(value)
}
impl Jobs {
    pub fn new() -> Self {
        let shell = process::getpid();
        let interactive = unsafe { bindings::isatty(0) } == 1;
        if interactive {
            unsafe {
                let ignore = Some(core::mem::transmute::<usize, unsafe extern "C" fn(i32)>(1));
                for signal in JOB_SIGNALS {
                    bindings::signal(signal, ignore);
                }
                bindings::setpgid(0, shell);
                bindings::tcsetpgrp(0, shell);
            }
        }
        Self {
            shell,
            interactive,
            next_id: 1,
            stopped: Vec::new(),
        }
    }
    pub fn foreground(&mut self, launch: Launch, label: String) -> i32 {
        if self.interactive && unsafe { bindings::tcsetpgrp(0, launch.group) } < 0 {
            launch.abort();
            return -1;
        }
        let last_pid = *launch.pids.last().unwrap();
        let job = Job {
            id: self.next_id,
            group: launch.group,
            members: launch
                .pids
                .into_iter()
                .map(|pid| Member {
                    pid,
                    stopped: false,
                })
                .collect(),
            label,
            modes: None,
            last_pid,
            status: 0,
        };
        self.next_id += 1;
        let saved = modes();
        let _ = io::close(launch.gate.0);
        let _ = io::close(launch.gate.1);
        self.wait(job, saved)
    }
    fn wait(&mut self, mut job: Job, saved: Option<bindings::termios>) -> i32 {
        while job.members.iter().any(|m| !m.stopped) {
            let mut status = 0;
            let pid = process::waitpid(-job.group, &mut status, WUNTRACED);
            if pid < 0 {
                if unsafe { *bindings::__errno_location() } == 4 {
                    continue;
                }
                break;
            }
            if status & 0xff == 0x7f {
                if let Some(member) = job.members.iter_mut().find(|m| m.pid == pid) {
                    member.stopped = true;
                }
                // A directed stop must stop the rest of a pipeline as well.
                unsafe {
                    bindings::kill(-job.group, 19);
                }
            } else {
                if pid == job.last_pid {
                    job.status = status;
                }
                job.members.retain(|m| m.pid != pid);
            }
        }
        if self.interactive {
            unsafe {
                job.modes = modes();
                bindings::tcsetpgrp(0, self.shell);
                if let Some(saved) = saved {
                    bindings::tcsetattr(0, 0, &saved);
                }
            }
        }
        if !job.members.is_empty() {
            println!("[{}] Stopped {}", job.id, job.label);
            self.stopped.push(job);
            -2
        } else if job.status & 0x7f != 0 {
            128 + (job.status & 0x7f)
        } else {
            (job.status >> 8) & 0xff
        }
    }
    pub fn reap(&mut self) {
        for job in &mut self.stopped {
            job.members.retain(|m| {
                let mut status = 0;
                process::waitpid(m.pid, &mut status, WNOHANG) == 0
            });
        }
        self.stopped.retain(|job| !job.members.is_empty());
    }
    pub fn list(&mut self) {
        self.reap();
        for job in &self.stopped {
            println!("[{}] Stopped {}", job.id, job.label);
        }
    }
    pub fn resume(&mut self, arg: Option<&str>) -> i32 {
        if process::getpid() != self.shell {
            println!("fg: requires the parent shell");
            return 1;
        }
        self.reap();
        let index = match arg {
            None => self.stopped.len().checked_sub(1),
            Some(id) => id
                .trim_start_matches('%')
                .parse::<usize>()
                .ok()
                .and_then(|id| self.stopped.iter().position(|job| job.id == id)),
        };
        let Some(index) = index else {
            println!("fg: no such job");
            return 1;
        };
        let mut job = self.stopped.remove(index);
        let saved = modes();
        if self.interactive {
            unsafe {
                if bindings::tcsetpgrp(0, job.group) < 0 {
                    self.stopped.insert(index, job);
                    return 1;
                }
                if let Some(modes) = job.modes {
                    bindings::tcsetattr(0, 0, &modes);
                }
            }
        }
        println!("{}", job.label);
        for member in &mut job.members {
            member.stopped = false;
        }
        unsafe {
            bindings::kill(-job.group, 18);
        }
        self.wait(job, saved)
    }
}
