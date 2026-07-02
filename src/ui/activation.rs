use smithay_client_toolkit::activation::{RequestData, RequestDataExt};

/// The data for requesting an activation token from the compositor while tracking
/// enough information for dispatching a corresponding org.freedesktop.Notifications.ActivationToken
/// event when the token is issued.
#[derive(Debug)]
pub struct ActivationRequestData {
    id: u32,
    action: Option<String>,
    data: RequestData,
}

impl ActivationRequestData {
    pub fn new(id: u32, action: Option<String>, data: RequestData) -> Self {
        Self { id, action, data }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn action(&self) -> Option<&String> {
        self.action.as_ref()
    }
}

impl RequestDataExt for ActivationRequestData {
    fn app_id(&self) -> Option<&str> {
        self.data.app_id()
    }

    fn seat_and_serial(&self) -> Option<(&wayland_client::protocol::wl_seat::WlSeat, u32)> {
        self.data.seat_and_serial()
    }

    fn surface(&self) -> Option<&wayland_client::protocol::wl_surface::WlSurface> {
        self.data.surface()
    }
}
