use smithay_client_toolkit::activation::{RequestData, RequestDataExt};

use crate::dbus::ServerMessage;

/// The data for requesting an activation token from the compositor while tracking
/// enough information for dispatching a corresponding org.freedesktop.Notifications.ActivationToken
/// event when the token is issued.
#[derive(Debug)]
pub struct ActivationRequestData {
    id: u32,
    data: RequestData,
}

impl ActivationRequestData {
    pub fn new(id: u32, data: RequestData) -> Self {
        Self { id, data }
    }

    pub fn server_message(&self, token: String) -> ServerMessage {
        ServerMessage::ActivationToken { id: self.id, token }
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
