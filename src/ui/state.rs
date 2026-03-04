/// The state of the application and its associated drivers for the
/// event loop.
pub struct State {
    width: u16,
    height: u16,

    done: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,

            done: false,
        }
    }
}
