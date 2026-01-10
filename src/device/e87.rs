use std::error::Error;

use crate::{
    ble_controller::{BleController, BleControllerError},
    device::{
        Connectable,
        common::{
            COMMAND_MARK, NOTIFY_CHARACTERISTIC_UUID, RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID,
        },
    },
    util::checksum,
};
use bitfield_struct::bitfield;
use bluest::{Characteristic, Device};
use eyre::eyre;
use futures::{Stream, StreamExt};
use uuid::Uuid;

pub struct E87Unconnected<'a> {
    pub(crate) device: Device,
    pub(crate) ble_controller: &'a BleController,
}

impl<'a> Connectable<E87<'a>> for E87Unconnected<'a> {
    async fn connect(self) -> Result<E87<'a>, BleControllerError> {
        self.ble_controller
            .adapter
            .connect_device(&self.device)
            .await?;
        return Ok(E87 {
            device: self.device,
            ble_controller: self.ble_controller,
        });
    }
}

#[bitfield(u8)] // In app code, known as flag status
pub struct PacketMetadata {
    /// Idk what this one does
    #[bits(1)]
    flag1: bool,

    /// In the app this is globally increasing counter
    #[bits(4)]
    packet_id: u8,

    /// Not sure what the point of this one is, I named it that
    /// as its only calculated that way in the app
    #[bits(1)]
    length_more_than_20: bool,

    /// Idk what this one does
    #[bits(1)]
    flag3: bool,

    /// Idk what this one does
    #[bits(1)]
    flag4: bool,
}

// Packet Layout
// | command_mark [1] | checksum [1] | packet metadata[1] | command [1] | length (LE) [2] | data_for_command [n] |

fn command_data(command: u8, data: &[u8]) -> Result<Vec<u8>, BleControllerError> {
    let total_length = data.len() + 6;
    let metadata = PacketMetadata::new()
        .with_packet_id(0)
        .with_flag1(false)
        .with_length_more_than_20(total_length > 20)
        .with_flag3(true)
        .with_flag4(false);

    let mut bytes = vec![
        metadata.into_bits(),
        command,
        (data.len() as u8) & 0xFF,
        ((data.len() >> 8) as u8) & 0xFF,
    ];

    bytes.extend_from_slice(data);
    bytes.splice(0..0, [COMMAND_MARK, checksum(data.iter().as_slice())]);

    debug_assert!(total_length == bytes.len());

    return Ok(bytes);
}

pub struct E87<'a> {
    device: Device,
    ble_controller: &'a BleController,
}

impl<'a> E87<'a> {
    async fn get_characteristic(
        &self,
        service: Uuid,
        characteristic: Uuid,
    ) -> Result<Characteristic, BleControllerError> {
        let services = self.device.discover_services_with_uuid(service).await?;
        let service = services.first().ok_or_else(|| eyre!(""))?;
        let mut characteristics = service
            .discover_characteristics_with_uuid(characteristic)
            .await?;
        let characteristic = characteristics.pop().ok_or_else(|| eyre!(""))?;
        Ok(characteristic)
    }

    pub async fn request_device_info(&self) -> Result<(), BleControllerError> {
        let write_char = self
            .get_characteristic(RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID)
            .await?;

        let notify_char = self
            .get_characteristic(RX_SERVICE_UUID, NOTIFY_CHARACTERISTIC_UUID)
            .await?;
        // let mut updates = notify_char.notify().await?;

        let s = async {
            while let Some(val) = updates.next().await {
                println!("notify state changed: {:?}", val?);
                break;
            }
            Ok::<(), BleControllerError>(())
        };

        //0xC6
        // characteristic
        let bytes = command_data(0xC6, &[1])?;
        write_char.write(bytes.as_slice()).await?;
        // s.await?;

        Ok(())
    }

    // Eventually implement drop etc...
    pub async fn disconnect(self) -> Result<(), BleControllerError> {
        self.ble_controller
            .adapter
            .disconnect_device(&self.device)
            .await?;
        Ok(())
    }
}
