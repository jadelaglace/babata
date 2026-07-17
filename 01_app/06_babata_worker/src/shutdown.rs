#[derive(Debug, Default, Clone, Copy)]
pub struct ShutdownSignal {
    requested: bool,
}

impl ShutdownSignal {
    pub fn shutdown(&mut self) {
        self.requested = true;
    }

    pub fn is_requested(self) -> bool {
        self.requested
    }
}
