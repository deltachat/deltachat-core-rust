use deltachat::location::Location;
use serde::Serialize;
use typescript_type_def::TypeDef;

#[derive(Serialize, TypeDef, schemars::JsonSchema)]
#[serde(rename = "Location", rename_all = "camelCase")]
pub struct JsonrpcLocation {
    pub location_id: u32,
    pub is_independent: bool,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
    pub timestamp: i64,
    pub contact_id: u32,
    pub msg_id: u32,
    pub chat_id: u32,
    pub marker: Option<String>,
}

impl From<Location> for JsonrpcLocation {
    fn from(location: Location) -> Self {
        let Location {
            location_id,
            independent,
            latitude,
            longitude,
            accuracy,
            timestamp,
            contact_id,
            msg_id,
            chat_id,
            marker,
        } = location;
        Self {
            location_id,
            is_independent: independent != 0,
            latitude,
            longitude,
            accuracy,
            timestamp,
            contact_id: contact_id.to_u32(),
            msg_id,
            chat_id: chat_id.to_u32(),
            marker,
        }
    }
}
