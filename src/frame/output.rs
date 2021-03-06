
#[derive(Clone, Debug)]
pub struct FrameOutput {
    pub exit: bool,
    pub swap_buffers: bool
}

impl Default for FrameOutput {
    fn default() -> Self {
        Self {
            exit: false,
            swap_buffers: true
        }
    }
}