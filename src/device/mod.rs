use crate::ble_controller::BleControllerError;

pub mod common;
pub mod e87;
pub enum BleDevice<'a> {
    E87(e87::E87Unconnected<'a>),
}

pub trait Connectable<T> {
    fn connect(self) -> impl std::future::Future<Output = Result<T, BleControllerError>>;
}
