use std::{error::Error, time::Duration};

use eyre::eyre;

use bluest::Adapter;
use futures::StreamExt;
use tokio::time::timeout;
use uuid::Uuid;

use crate::device::{common::FILTER_UUID, e87::E87Unconnected};

pub type BleControllerError = Box<dyn Error>;

pub struct BleController {
    pub(crate) adapter: Adapter,
}

pub enum UnconnectedDevice<'a> {
    E87(E87Unconnected<'a>),
}

impl BleController {
    pub async fn new() -> Result<BleController, BleControllerError> {
        let controller = BleController {
            adapter: Adapter::default().await.ok_or_else(|| eyre!("err"))?,
        };
        Ok(controller)
    }

    // TODO: optional timeouts
    pub async fn wait_for_available(
        &self,
        timeout_length: Duration,
    ) -> Result<(), BleControllerError> {
        Ok(timeout(timeout_length, self.adapter.wait_available()).await??)
    }

    // TODO: optional timeouts
    pub async fn scan_for_supported_devices(
        &self,
    ) -> Result<UnconnectedDevice, BleControllerError> {
        let mut scan = self.adapter.scan(&[]).await?;
        while let Some(discovered_device) = scan.next().await {
            let badge_service_present = discovered_device.adv_data.services.contains(&FILTER_UUID);

            // TODO: Ideally here we would check for some other UUIDs contained within the advertising data
            //       but this BLE lib does not allow access to all the things in advertising data :sob:
            if !badge_service_present {
                continue;
            }

            println!(
                "Found Device: '{}' [{}]",
                discovered_device
                    .device
                    .name_async()
                    .await
                    .unwrap_or("unknown".to_string()),
                discovered_device.device.id()
            );
            return Ok(UnconnectedDevice::E87(E87Unconnected {
                device: discovered_device.device,
                ble_controller: self,
            }));
        }
        Err(eyre!("").into())
    }
}
