use std::{error::Error, io::Cursor};

use binrw::{BinRead, binrw};
use bitfield_struct::bitfield;
use bluest::{Characteristic, Device};
use eyre::eyre;
use futures::StreamExt;
use num_traits::{ToPrimitive, ops::bytes};
use uuid::Uuid;

use crate::{
    ble_controller::{BleController, BleControllerError},
    device::{
        Connectable,
        command::RequestCommand,
        common::{
            COMMAND_MARK, CONTROL_POINT_CHARACTERISTIC_UUID, NOTIFY_CHARACTERISTIC_UUID,
            RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID,
        },
    },
    util::{Counter, JdkHash, checksum},
};

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

#[bitfield(u8, order = Msb)]
pub struct BindRequestInfo {
    /// Idk what this one does
    #[bits(5, default = 0)]
    unknown1: u8,

    #[bits(1, default = false)]
    is_using_12_hour_format: bool,

    #[bits(1, default = true)]
    is_not_using_zh_locale: bool,

    #[bits(1, default = 0)]
    unknown2: u8,
}

// Packet Layout
// | command_mark [1] | checksum [1] | packet metadata[1] | command [1] | length (LE) [2] | data_for_command [n]       |
// | 9E               | 01           | 40 [0b1000000]     | C6          | 00 01           | 01                         |
// | 9E               | 2a           | 62 [0b1100010]     | C6          | 00 01           | 01                         |
// | 9e               | 8f           | 62                 | 60          | 0d 00           | 06cfc172ddffffcfc172ddffff |

// Bind request
// byte[0] = | reserved [5] | isNotUsing24HourFormat [1] | isNotUsingChineseLanguage [1] | reserved [1] |
// byte[1..7] = device info hash first 6 bytes
// byte[7..13] = device info hash first 6 bytes again ????

// deviceProperties: u32 = jdkHashCode(
//     "35${boardLen % 10}${brandLen % 10}${cpuAbiLen % 10}${deviceLen % 10}${displayLen % 10}${hostLen % 10}${idLen % 10}${manuLen % 10}${modelLen % 10}${productLen % 10}${tagsLen % 10}${typeLen % 10}${userLen % 10}"
// );
// idPacketData = [0; 13];
// idPacketData[0] = 0b00000110; // | reserved [5] | isNotUsing24HourFormat [1] | isNotUsingChineseLanguage [1] | reserved [1] |
// idPacketData[1..7] = u64To6BytesLE(deviceProperties);
// idPacketData[7..13] = u64To6BytesLE(deviceProperties);

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

    pub async fn request_device_info(&mut self) -> Result<BadgeInfo, BleControllerError> {
        const REQ_COMMAND: u8 = 0xC6;
        const RES_COMMAND: u8 = 0xC7;

        let write_char = self
            .get_characteristic(RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID)
            .await?;

        let notify_char = self
            .get_characteristic(RX_SERVICE_UUID, NOTIFY_CHARACTERISTIC_UUID)
            .await?;
        let mut updates = notify_char.notify().await?;

        let badge_info = async {
            while let Some(bytes) = updates.next().await {
                return parse_command_data_response(
                    bytes?.as_slice(),
                    Some(&[RES_COMMAND]),
                    |_, _, data| {
                        if data[0] != 1 {
                            // I error here, the official app seems to return an empty badge info object
                            return Err(eyre!("").into());
                        }
                        Ok(BadgeInfo::read(&mut Cursor::new(&data[1..]))?)
                    },
                );
            }
            Err(eyre!("").into())
        };

        //0xC6
        // characteristic
        let bytes = command_data(self.packet_counter.next(), REQ_COMMAND, &[1])?;
        println!("REQUEST_DEVICE_INFO: {:?}", bytes);
        write_char.write(bytes.as_slice()).await?;
        badge_info.await
    }

    // TODO: When binding for the first time (or after an unbind, there will be 3 NOTIFICATIONS
    //       received, for firsttime bond, the first notif is just a single data byte (2) :shrug:)
    pub async fn request_bind(&mut self) -> Result<(), BleControllerError> {
        const REQ_COMMAND: u8 = 0x60;
        const RES_COMMAND: u8 = 0x61;

        let write_char = self
            .get_characteristic(RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID)
            .await?;

        let notify_char = self
            .get_characteristic(RX_SERVICE_UUID, NOTIFY_CHARACTERISTIC_UUID)
            .await?;
        let mut updates = notify_char.notify().await?;

        let response = async {
            let mut packet_index = 0u8;
            let mut expected_packet_count = 2;
            while let Some(bytes) = updates.next().await {
                let bytes = bytes?;
                if packet_index == 0 && bytes.len() == 7 && *bytes.last().unwrap() == 2 {
                    expected_packet_count = 3;
                }

                if packet_index == 0 || (packet_index == 1 && expected_packet_count == 3) {
                    _ = parse_command_data_response(
                        bytes.as_slice(),
                        Some(&[RES_COMMAND]),
                        |_, _, data| {
                            println!("BIND: {:?}", data);
                            Ok(())
                        },
                    )?;
                } else if (packet_index == 1 && expected_packet_count == 2) || packet_index == 2 {
                    println!("final_packet: {:02X?}", bytes);
                }
                packet_index += 1;
                if packet_index >= expected_packet_count {
                    return Ok(());
                }
            }
            Err(eyre!("").into())
        };

        let device_info_str = format!(
            "35{buildBoard}{buildBrand}{buildCpuAbi}{buildDevice}{buildDisplay}{buildHost}{buildId}{buildManu}{buildMode}{buildModel}{buildProduct}{buildTags}{buildType}{buildUser}",
            buildBoard = "my board".len() % 10,
            buildBrand = "my brand".len() % 10,
            buildCpuAbi = "my cpuAbi".len() % 10,
            buildDevice = "my device".len() % 10,
            buildDisplay = "my display".len() % 10,
            buildHost = "my host".len() % 10,
            buildId = "my id".len() % 10,
            buildManu = "my manu".len() % 10,
            buildMode = "my mode".len() % 10,
            buildModel = "my model".len() % 10,
            buildProduct = "my product".len() % 10,
            buildTags = "my tags".len() % 10,
            buildType = "my type".len() % 10,
            buildUser = "my user".len() % 10,
        );
        println!("device_info_str: {}", device_info_str);
        let device_info_hash = device_info_str
            .jdk_hash_code()
            .to_i64()
            .unwrap()
            .to_le_bytes();
        let data = &[
            BindRequestInfo::new().into_bits(),
            device_info_hash[0],
            device_info_hash[1],
            device_info_hash[2],
            device_info_hash[3],
            device_info_hash[4],
            device_info_hash[5],
            device_info_hash[0],
            device_info_hash[1],
            device_info_hash[2],
            device_info_hash[3],
            device_info_hash[4],
            device_info_hash[5],
        ];
        let bytes = command_data(self.packet_counter.next(), REQ_COMMAND, data)?;
        println!("BIND REQ: {:02X?}", bytes);
        write_char.write(bytes.as_slice()).await?;
        response.await
    }

    pub async fn request_unbind(&mut self) -> Result<(), BleControllerError> {
        const REQ_COMMAND: u8 = 0x62;
        const RES_COMMAND: u8 = 0xff;

        let write_char = self
            .get_characteristic(RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID)
            .await?;

        let notify_char = self
            // .get_characteristic(RX_SERVICE_UUID, NOTIFY_CHARACTERISTIC_UUID)
            .get_characteristic(RX_SERVICE_UUID, CONTROL_POINT_CHARACTERISTIC_UUID)
            // .get_characteristic(RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID)
            .await?;
        let mut updates = notify_char.notify().await?;

        let response = async {
            while let Some(bytes) = updates.next().await {
                println!("{:?}", bytes);
                return parse_command_data_response(
                    bytes?.as_slice(),
                    Some(&[RES_COMMAND]),
                    |_, _, data| {
                        println!("UNBIND: {:?}", data);
                        Ok(())
                    },
                );
            }
            Err(eyre!("").into())
        };

        let bytes = command_data(self.packet_counter.next(), REQ_COMMAND, &[1])?;
        println!("UNBIND REQ: {:02X?}", bytes);
        write_char
            .write(&[0x9e, 0x86, 0x22, 0x62, 0x01, 0x00, 0x01])
            .await?;
        response.await
    }

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
pub struct BadgeInfo {
    width: u16,
    height: u16,
    picture_width: u16,
    picture_height: u16,
    memory: u32,
}

pub fn parse_command_data_response<T>(
    bytes: &[u8],
    allowed_commands: Option<&[u8]>,
    parse_data: impl Fn(PacketMetadata, u8, &[u8]) -> Result<T, BleControllerError>,
) -> Result<T, BleControllerError> {
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

    // NOTE: The official app does not parse the packet if actual length
    //       is > 20, idk why, but I just parse regardless
    let actual_length = bytes.len() - 6;
    if reported_length != actual_length as u16 {
        println!(
            "reported length [{}] does not match actual length [{}]",
            reported_length, actual_length
        );
    }

    let actual_checksum = checksum(&bytes[2..]);
    if reported_checksum != actual_checksum {
        println!(
            "reported checksum [{}] does not match actual checksum [{}]",
            reported_checksum, actual_checksum
        );
    }

    let data = &bytes[6..];

    if packet_metadata.expect_response() {
        todo!("IDK YET???")
        // this.commandManager.command_a2d_sendResponse(command, true);
    }

    if let Some(allowed_commands) = allowed_commands
        && !allowed_commands.contains(&command)
    {
        return Err(eyre!("").into());
    }

    if let Ok(parsed) = parse_data(packet_metadata, command, data) {
        return Ok(parsed);
    }
    Err(eyre!("").into())
}
