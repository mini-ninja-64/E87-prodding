use uuid::{Uuid, uuid};
pub const COMMAND_MARK: u8 = 0x9E;

pub const FILTER_UUID: Uuid = uuid!("0000fd00-0000-1000-8000-00805f9b34fb");

pub const RX_SERVICE_UUID: Uuid = uuid!("c2e6fd00-e966-1000-8000-bef9c223df6a");

// 0x000E
pub const NOTIFY_CHARACTERISTIC_UUID: Uuid = uuid!("c2e6fd01-e966-1000-8000-bef9c223df6a");

// 0x000C
pub const WRITE_CHARACTERISTIC_UUID: Uuid = uuid!("c2e6fd02-e966-1000-8000-bef9c223df6a");
// 0x0011
pub const CONTROL_POINT_CHARACTERISTIC_UUID: Uuid = uuid!("c2e6fd03-e966-1000-8000-bef9c223df6a");

pub const UNKNOWN_CHARACTERISTIC_UUID_1: Uuid = uuid!("c2e6fd04-e966-1000-8000-bef9c223df6a");
pub const UNKNOWN_CHARACTERISTIC_UUID_2: Uuid = uuid!("c2e6fd05-e966-1000-8000-bef9c223df6a");
