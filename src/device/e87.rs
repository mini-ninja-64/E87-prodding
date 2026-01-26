use std::error::Error;

use crate::{
    ble_controller::{BleController, BleControllerError},
    device::{
        Connectable,
        command::RequestCommand,
        common::{
            COMMAND_MARK, NOTIFY_CHARACTERISTIC_UUID, RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID,
        },
    },
    util::{Counter, checksum},
};
use binrw::{BinRead, binrw};
use bitfield_struct::bitfield;
use bluest::{Characteristic, Device};
use eyre::eyre;
use futures::{Stream, StreamExt};
use std::io::Cursor;
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
            packet_counter: Counter::new(0, 1, 16),
        });
    }
}

#[bitfield(u8, order = Msb)] // In app code, known as flag status
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

    #[bits(1)]
    expect_response: bool,

    /// Idk what this one does
    #[bits(1)]
    flag4: bool,
}

// Packet Layout
// | 9E               | 01           | 40 [0b1000000]     | C6          | 00 01           | 01                   |
// | 9E               | 2a           | 62 [0b1100010]     | C6          | 00 01           | 01                   |
// | command_mark [1] | checksum [1] | packet metadata[1] | command [1] | length (LE) [2] | data_for_command [n] |

#[test]
fn test_packet_metadata() {
    let metadata = PacketMetadata::new()
        .with_packet_id(12)
        .with_flag1(false)
        .with_length_more_than_20(false)
        .with_expect_response(true)
        .with_flag4(false)
        .into_bits();
    assert_eq!(metadata, 0x62)
}

fn command_data(packet_id: u8, command: u8, data: &[u8]) -> Result<Vec<u8>, BleControllerError> {
    let total_length = data.len() + 6;
    let metadata = PacketMetadata::new()
        .with_packet_id(packet_id)
        .with_flag1(false)
        .with_length_more_than_20(total_length > 20)
        .with_expect_response(true)
        .with_flag4(false);

    let mut bytes = vec![
        metadata.into_bits(),
        command,
        (data.len() as u8) & 0xFF,
        ((data.len() >> 8) as u8) & 0xFF,
    ];

    bytes.extend_from_slice(data);

    let checksum = checksum(&bytes.iter().as_slice());

    bytes.splice(0..0, [COMMAND_MARK, checksum]);

    debug_assert!(total_length == bytes.len());

    return Ok(bytes);
}

pub struct E87<'a> {
    device: Device,
    ble_controller: &'a BleController,
    packet_counter: Counter<u8>,
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

    pub async fn request_device_info(&mut self) -> Result<(), BleControllerError> {
        let write_char = self
            .get_characteristic(RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID)
            .await?;

        let notify_char = self
            .get_characteristic(RX_SERVICE_UUID, NOTIFY_CHARACTERISTIC_UUID)
            .await?;
        let mut updates = notify_char.notify().await?;

        let s = async {
            while let Some(val) = updates.next().await {
                parse_response(&val?).unwrap();
                //  [9E, 18, 00, C7, 0D, 00, 01, 68, 01, 68, 01, 70, 01, 70, 01, 80, 0F, 00, 00] packet_id == 0
                //  [9E, 18, 00, C7, 0D, 00, 01, 68, 01, 68, 01, 70, 01, 70, 01, 80, 0F, 00, 00] packet_id == 12
                break;
            }
            Ok::<(), BleControllerError>(())
        };

        //0xC6
        // characteristic
        let bytes = command_data(
            self.packet_counter.next(),
            RequestCommand::BadgeInfo.into(),
            &[1],
        )?;
        println!("{:02X?}", bytes.as_slice());

        write_char.write(bytes.as_slice()).await?;
        s.await?;

        Ok(())
    }

    // pub async fn request_image_size(&mut self) -> Result<(), BleControllerError> {
    //     let write_char = self
    //         .get_characteristic(RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID)
    //         .await?;

    //     let notify_char = self
    //         .get_characteristic(RX_SERVICE_UUID, NOTIFY_CHARACTERISTIC_UUID)
    //         .await?;
    //     let mut updates = notify_char.notify().await?;

    //     let s = async {
    //         while let Some(val) = updates.next().await {
    //             println!("notify state changed: {:02X?}", val?);
    //             break;
    //         }
    //         Ok::<(), BleControllerError>(())
    //     };

    //     let bytes = command_data(
    //         self.packet_counter.next(),
    //         RequestCommand::PictureSize.into(),
    //         &[],
    //     )?;
    //     println!("{:02X?}", bytes.as_slice());

    //     write_char.write(bytes.as_slice()).await?;
    //     s.await?;

    //     Ok(())
    // }

    pub async fn disconnect(self) -> Result<(), BleControllerError> {
        self.ble_controller
            .adapter
            .disconnect_device(&self.device)
            .await?;
        Ok(())
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug)]
struct BadgeInfo {
    width: u16,
    height: u16,
    picture_width: u16,
    picture_height: u16,
    memory: u32,
}

pub fn parse_response(bytes: &[u8]) -> Result<(), BleControllerError> {
    println!("parsing response: {:02X?}", bytes);
    if bytes.len() <= 6 {
        return Err(eyre!("").into());
    }
    let _command_mark = bytes[0];
    let reported_checksum = bytes[1];
    let packet_metadata = PacketMetadata::from_bits(bytes[2]);
    let command = bytes[3];
    let reported_length: u16 = u16::from_le_bytes([bytes[4], bytes[5]]);

    println!("response metadata: {:?}", packet_metadata);

    let actual_length = bytes.len() - 6;
    debug_assert_eq!(reported_length, actual_length.try_into()?);
    let actual_checksum = checksum(&bytes[2..]);
    debug_assert_eq!(reported_checksum, actual_checksum);

    let data = &bytes[6..];

    if packet_metadata.expect_response() {
        todo!("IDK YET???")
        // this.commandManager.command_a2d_sendResponse(command, true);
    }

    println!("parsing badge info response");

    match command {
        0xC7 => {
            if data[0] != 1 {
                // I error here, the official app seems to return an empty badge info object
                return Err(eyre!("").into());
            }
            println!("parsing badge info response");
            let badge_info = BadgeInfo::read(&mut Cursor::new(&data[1..]))?;
            println!("{:?}", badge_info);
        }
        _ => return Err(eyre!("").into()),
    }

    Ok(())
}
