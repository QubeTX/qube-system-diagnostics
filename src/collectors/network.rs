use serde::Serialize;
use sysinfo::Networks;

use crate::observation::Observation;

#[derive(Debug, Clone, Default, Serialize)]
pub struct NetworkData {
    pub interfaces: Vec<InterfaceInfo>,
    pub total_download_rate: u64,
    pub total_upload_rate: u64,
    pub adapters: Vec<NetworkAdapterInfo>,
    pub adapter_status: Observation,
}

#[derive(Debug, Clone, Serialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub ip_addresses: Vec<String>,
    pub mac_address: String,
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
    pub download_rate: u64,
    pub upload_rate: u64,
    pub is_up: bool,
    pub operational_state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkAdapterInfo {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub media_connection_state: Option<u32>,
    pub link_speed_bps: Option<u64>,
    pub hardware_interface: Option<bool>,
}

pub fn collect(networks: &mut Networks) -> NetworkData {
    networks.refresh(true);

    let interfaces: Vec<InterfaceInfo> = networks
        .iter()
        .map(|(name, data)| {
            let operational_state = data.operational_state().to_string();
            let normalized_state = operational_state.to_ascii_lowercase();
            let is_up = matches!(
                normalized_state.as_str(),
                "up" | "unknown" | "dormant" | "lowerlayerdown"
            );

            let ip_addresses: Vec<String> = data
                .ip_networks()
                .iter()
                .map(|ip| ip.addr.to_string())
                .collect();

            InterfaceInfo {
                name: name.clone(),
                ip_addresses,
                mac_address: data.mac_address().to_string(),
                received_bytes: data.total_received(),
                transmitted_bytes: data.total_transmitted(),
                download_rate: data.received(),
                upload_rate: data.transmitted(),
                is_up,
                operational_state,
            }
        })
        .collect();

    let total_download_rate = interfaces.iter().map(|iface| iface.download_rate).sum();
    let total_upload_rate = interfaces.iter().map(|iface| iface.upload_rate).sum();

    NetworkData {
        interfaces,
        total_download_rate,
        total_upload_rate,
        adapters: Vec::new(),
        adapter_status: Observation::default(),
    }
}

pub fn refresh_hardware(data: &mut NetworkData) {
    #[cfg(windows)]
    {
        let (adapters, status) = collect_windows_adapters();
        data.adapters = adapters;
        data.adapter_status = status;
    }

    #[cfg(not(windows))]
    {
        data.adapter_status = Observation::unsupported(
            "platform network adapter provider",
            "Static adapter capabilities are not implemented on this platform",
        );
    }
}

#[cfg(windows)]
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiNetAdapter {
    name: Option<String>,
    interface_description: Option<String>,
    interface_operational_status: Option<u32>,
    media_connect_state: Option<u32>,
    receive_link_speed: Option<u64>,
    transmit_link_speed: Option<u64>,
    hardware_interface: Option<bool>,
}

#[cfg(windows)]
fn collect_windows_adapters() -> (Vec<NetworkAdapterInfo>, Observation) {
    use wmi::{COMLibrary, WMIConnection};

    let com = match COMLibrary::new() {
        Ok(com) => com,
        Err(error) => {
            return (
                Vec::new(),
                Observation::error("MSFT_NetAdapter", format!("COM init failed: {error}")),
            )
        }
    };
    let connection = match WMIConnection::with_namespace_path("root\\StandardCimv2", com) {
        Ok(connection) => connection,
        Err(error) => {
            return (
                Vec::new(),
                Observation::error("MSFT_NetAdapter", format!("WMI connection failed: {error}")),
            )
        }
    };
    let rows = match connection.raw_query::<WmiNetAdapter>(
        "SELECT Name, InterfaceDescription, InterfaceOperationalStatus, MediaConnectState, ReceiveLinkSpeed, TransmitLinkSpeed, HardwareInterface FROM MSFT_NetAdapter",
    ) {
        Ok(rows) => rows,
        Err(error) => {
            return (
                Vec::new(),
                Observation::error("MSFT_NetAdapter", format!("WMI query failed: {error}")),
            )
        }
    };
    let adapters = rows
        .into_iter()
        .filter(|row| row.hardware_interface != Some(false))
        .filter_map(|row| {
            let name = row.name?;
            Some(NetworkAdapterInfo {
                name,
                description: clean_string(row.interface_description),
                status: row.interface_operational_status.map(|status| match status {
                    1 => "Up".into(),
                    2 => "Down".into(),
                    6 => "Not present".into(),
                    value => format!("Status {value}"),
                }),
                media_connection_state: row.media_connect_state,
                link_speed_bps: row.receive_link_speed.max(row.transmit_link_speed),
                hardware_interface: row.hardware_interface,
            })
        })
        .collect::<Vec<_>>();
    let status = if adapters.is_empty() {
        Observation::unavailable(
            "MSFT_NetAdapter",
            "The provider returned no hardware network adapters",
        )
    } else {
        Observation::available("MSFT_NetAdapter")
    };
    (adapters, status)
}

#[cfg(windows)]
fn clean_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}
