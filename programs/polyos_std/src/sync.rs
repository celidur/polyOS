#[derive(Clone, Copy)]
pub struct Semaphore {
    id: i32,
}

impl Semaphore {
    pub fn create(initial_count: i32) -> Result<Self, i32> {
        let id = unsafe { crate::bindings::sem_create(initial_count) };

        if id >= 0 {
            Ok(Self { id })
        } else {
            Err(id)
        }
    }

    pub fn id(self) -> i32 {
        self.id
    }

    pub fn wait(self) -> Result<(), i32> {
        let result = unsafe { crate::bindings::sem_wait(self.id) };

        if result == 0 {
            Ok(())
        } else {
            Err(result)
        }
    }

    pub fn signal(self) -> Result<(), i32> {
        let result = unsafe { crate::bindings::sem_signal(self.id) };

        if result == 0 {
            Ok(())
        } else {
            Err(result)
        }
    }

    pub fn close(self) -> Result<(), i32> {
        let result = unsafe { crate::bindings::sem_close(self.id) };

        if result == 0 {
            Ok(())
        } else {
            Err(result)
        }
    }
}
