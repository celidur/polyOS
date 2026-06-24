#[derive(Clone, Copy)]
pub struct Semaphore {
    sem: crate::bindings::sem_t,
}

impl Semaphore {
    pub fn create(initial_count: i32) -> Result<Self, i32> {
        let mut sem = 0 as crate::bindings::sem_t;
        let result = unsafe { crate::bindings::sem_init(&mut sem, 0, initial_count as u32) };

        if result == 0 {
            Ok(Self { sem })
        } else {
            Err(result)
        }
    }

    pub fn id(self) -> i32 {
        self.sem as i32
    }

    pub fn wait(self) -> Result<(), i32> {
        let mut sem = self.sem;
        let result = unsafe { crate::bindings::sem_wait(&mut sem) };

        if result == 0 {
            Ok(())
        } else {
            Err(result)
        }
    }

    pub fn signal(self) -> Result<(), i32> {
        let mut sem = self.sem;
        let result = unsafe { crate::bindings::sem_post(&mut sem) };

        if result == 0 {
            Ok(())
        } else {
            Err(result)
        }
    }

    pub fn close(self) -> Result<(), i32> {
        let mut sem = self.sem;
        let result = unsafe { crate::bindings::sem_destroy(&mut sem) };

        if result == 0 {
            Ok(())
        } else {
            Err(result)
        }
    }
}
