use bluest::btuuid::characteristics;
use bluest::{Adapter, Characteristic, Device};
use eyre::eyre;
// use btleplug::api::{
//     Central, Manager as _, Peripheral as _, ScanFilter, WriteType, bleuuid::uuid_from_u16,
// };
// use btleplug::api::{CentralEvent, Peripheral};
// use btleplug::platform::{Adapter, Manager};
use rand::{Rng, rng};
use std::any::Any;
use std::error::Error;
use std::iter::Sum;
use std::num::Wrapping;
use std::time::Duration;
use std::{thread, vec};
use tokio::time;
use uuid::{Uuid, uuid};

// use btleplug::api::{Manager as _, bleuuid::BleUuid};
use futures::stream::StreamExt;

// const LIGHT_CHARACTERISTIC_UUID: Uuid = uuid_from_u16(0xFFE9);
const RX_SERVICE_UUID: Uuid = uuid!("c2e6fd00-e966-1000-8000-bef9c223df6a");
const NOTIFY_CHARACTERISTIC_UUID: Uuid = uuid!("c2e6fd01-e966-1000-8000-bef9c223df6a");
const WRITE_CHARACTERISTIC_UUID: Uuid = uuid!("c2e6fd02-e966-1000-8000-bef9c223df6a");
const CONTROL_POINT_CHARACTERISTIC_UUID: Uuid = uuid!("c2e6fd03-e966-1000-8000-bef9c223df6a");

const UNKNOWN_CHARACTERISTIC_UUID_1: Uuid = uuid!("c2e6fd04-e966-1000-8000-bef9c223df6a");
const UNKNOWN_CHARACTERISTIC_UUID_2: Uuid = uuid!("c2e6fd05-e966-1000-8000-bef9c223df6a");

const COMMAND_MARK: u8 = 0x9E;

fn checksum(data: &[u8]) -> u8 {
    // return data.iter().cloned().map(Wrapping).sum::<Wrapping<u8>>().0;
    let mut value = 0u8;
    for n in data {
        value = value.wrapping_add(*n);
    }
    return value;
}

fn flag_status(b1: bool, b2: bool, b3: bool, b4: bool) -> u8 {
    /* TODO:
     */
    return 0;
}

fn uuids_from_advertising_data(data: &[u8]) {}
// | command_mark [1] | checksum [1] | flag_status[1] | command [1] | length (LE) [2] | data_for_command [n] |

fn command_data(command: u8, data: &[u8]) -> Vec<u8> {
    let ir_len = data.len() + 4;
    let mut ir_arr = vec![0u8; ir_len];
    ir_arr[0] = flag_status(false, data.len() + 6 > 20, true, false);
    ir_arr[1] = command;
    ir_arr[2] = (data.len() as u8) & 255;
    ir_arr[3] = ((data.len() >> 8) as u8) & 255;
    // TODO: ir_arr[4..] = data[0..];
    // let final_arr = [COMMAND_MARK, checksum(&ir_arr), ...ir_arr];
    // return final_arr;
    vec![]
}

struct Badge {
    device: Device,
}

impl Badge {
    async fn get_characteristic(
        &self,
        service: Uuid,
        characteristic: Uuid,
    ) -> Result<Characteristic, Box<dyn Error>> {
        let services = self.device.discover_services_with_uuid(service).await?;
        let service = services.first().ok_or_else(|| eyre!(""))?;
        let mut characteristics = service
            .discover_characteristics_with_uuid(characteristic)
            .await?;
        let characteristic = characteristics.pop().ok_or_else(|| eyre!(""))?;
        Ok(characteristic)
    }

    pub async fn request_device_info(&self) -> Result<(), Box<dyn Error>> {
        let characteristic = self
            .get_characteristic(RX_SERVICE_UUID, WRITE_CHARACTERISTIC_UUID)
            .await?;

        // characteristic
        let bytes = command_data(0xC6, &[1]);
        characteristic.write(bytes.as_slice()).await?;

        Ok(())
    }
}

const FILTER_UUID: Uuid = uuid!("0000fd00-0000-1000-8000-00805f9b34fb");

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::default()
        .await
        .ok_or("Bluetooth adapter not found")?;
    println!("Turn on BLE plz");
    adapter.wait_available().await?;

    println!("starting scan");
    let mut scan = adapter.scan(&[]).await?;
    println!("scan started");
    while let Some(discovered_device) = scan.next().await {
        let badge_service_present = discovered_device.adv_data.services.contains(&FILTER_UUID);
        if !badge_service_present {
            continue;
        }
        // TODO: Ideally here we would check for some other UUIDs but this BLE lib does not allow
        //       access to all the things in advertising data :sob:

        println!(
            "Found badge: '{}' [{}]",
            discovered_device
                .device
                .name_async()
                .await
                .unwrap_or("unknown".to_string()),
            discovered_device.device.id()
        );

        // Connect to badge
        adapter.connect_device(&discovered_device.device).await?;

        let device = discovered_device.device;
        let full_services = device.discover_services().await?;
        let rx_svc = full_services.iter().find_map(|svc| {
            if svc.uuid() == RX_SERVICE_UUID {
                Some(svc)
            } else {
                None
            }
        });

        let badge = Badge { device: device };
        badge.request_device_info().await?;

        break;
    }
    Ok(())
}

// #[tokio::main]
//     async fn main() -> Result<(), Box<dyn Error>> {
//         let manager = Manager::new().await?;

//         // get the first bluetooth adapter
//         let adapters = manager.adapters().await?;
//         let central = adapters.into_iter().nth(0).unwrap();

//         // event stream
//         let mut events = central.events().await?;

//         // start scanning for devices
//         central.start_scan(ScanFilter::default()).await?;

//         while let Some(event) = events.next().await {
//             match event {
//                 CentralEvent::DeviceDiscovered(id) => {
//                     let peripheral = central.peripheral(&id).await?;
//                     let properties = peripheral.properties().await?.unwrap();
//                     // let name = properties
//                     //     .and_then(|p| p.local_name)
//                     //     .map(|local_name| format!("Name: {local_name}"))
//                     //     .unwrap_or_default();

//                     println!(
//                         "Device Discovered: {:?} [{:?}]",
//                         properties.local_name, properties.manufacturer_data
//                     );
//                 }
//                 CentralEvent::StateUpdate(state) => {
//                     println!("AdapterStatusUpdate {:?}", state);
//                 }
//                 CentralEvent::DeviceConnected(id) => {
//                     let periph = central.peripheral(&id).await?;
//                     periph.discover_services().await?;
//                     println!("DeviceConnected: {:?}", id);
//                     let services = periph.services();

//                     // for svc in services {
//                     //     println!("service: {}, primary: {}", svc.uuid, svc.primary);
//                     //     println!("characteristics: {:?}", svc.characteristics);
//                     //     println!("");
//                     // }
//                     let badge_svc = services.iter().filter(|s| s.uuid == RX_SERVICE_UUID).next();

//                     if badge_svc.is_none() {
//                         println!("no badge svc: {:?}", services);
//                         continue;
//                     }
//                     println!("Found badge service :3 [{}]", periph.address());
//                     let badge_svc = badge_svc.unwrap();
//                     let characteristics: &std::collections::BTreeSet<btleplug::api::Characteristic> =
//                         &badge_svc.characteristics;

//                     for char in characteristics {
//                         println!(
//                             "characteristic: {}, properties: {:?}",
//                             char.uuid, char.properties
//                         );
//                         println!("descriptors: {:?}", char.descriptors);
//                         println!("");
//                         // if char.uuid == WRITE_CHARACTERISTIC_UUID {
//                         //     periph.write(char, &[0], WriteType::WithResponse).await?;
//                         // }
//                     }
//                 }
//                 CentralEvent::DeviceDisconnected(id) => {
//                     println!("DeviceDisconnected: {:?}", id);
//                 }
//                 CentralEvent::ManufacturerDataAdvertisement {
//                     id,
//                     manufacturer_data,
//                 } => {
//                     // println!("id: {:?}, manu_data: {:?}", id, manufacturer_data)
//                 }
//                 CentralEvent::ServiceDataAdvertisement { id, service_data } => {
//                     // println!("id: {:?}, service_data: {:?}", id, service_data)
//                 }
//                 CentralEvent::ServicesAdvertisement { id, services } => {
//                     let services: Vec<String> =
//                         services.into_iter().map(|s| s.to_short_string()).collect();
//                     // println!("[{:?}] Services advertisement", id)
//                 }
//                 CentralEvent::DeviceUpdated(id) => {
//                     // println!("DeviceUpdated: {:?}", id);
//                 }
//             }
//         }

//         Ok(())

//         // // find the device we're interested in
//         // let light = find_light(&central).await.unwrap();

//         // // connect to the device
//         // light.connect().await?;

//         // // discover services and characteristics
//         // light.discover_services().await?;

//         // // find the characteristic we want
//         // let chars = light.characteristics();
//         // let cmd_char = chars
//         //     .iter()
//         //     .find(|c| c.uuid == LIGHT_CHARACTERISTIC_UUID)
//         //     .unwrap();

//         // // dance party
//         // let mut rng = rng();
//         // for _ in 0..20 {
//         //     let color_cmd = vec![
//         //         0x56,
//         //         rng.random(),
//         //         rng.random(),
//         //         rng.random(),
//         //         0x00,
//         //         0xF0,
//         //         0xAA,
//         //     ];
//         //     light
//         //         .write(&cmd_char, &color_cmd, WriteType::WithoutResponse)
//         //         .await?;
//         //     time::sleep(Duration::from_millis(200)).await;
//         // }
//         // Ok(())
//     }

// async fn find_light(central: &Adapter) -> Option<Peripheral> {
//     for p in central.peripherals().await.unwrap() {
//         if p.properties()
//             .await
//             .unwrap()
//             .unwrap()
//             .local_name
//             .iter()
//             .any(|name| name.contains("LEDBlue"))
//         {
//             return Some(p);
//         }
//     }
//     None
// }
