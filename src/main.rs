pub mod ble_controller;
pub mod device;
pub mod util;

use bitfield_struct::bitfield;
use std::time::Duration;
use std::vec;

use crate::{
    ble_controller::{BleController, BleControllerError, UnconnectedDevice},
    device::Connectable,
};

// Sometimes its called "E87_Audio" ?????????

#[tokio::main]
async fn main() -> Result<(), BleControllerError> {
    let controller = BleController::new().await?;
    controller
        .wait_for_available(Duration::from_secs(10))
        .await?;

    println!("starting scan");
    let device = controller.scan_for_supported_devices().await?;
    match device {
        UnconnectedDevice::E87(device) => {
            let device = device.connect().await?;

            println!("Requesting");
            device.request_device_info().await?;

            println!("Disconnecting");
            device.disconnect().await?;
        }
    }
    Ok(())
}
