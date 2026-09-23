use serde::Serialize;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use sysinfo::Networks;

use super::sampling::{CounterRate, SampleMeta};

use crate::observation::Observation;

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct NetworkData {
    #[serde(default)]
    pub sample: SampleMeta,
    pub interfaces: Vec<InterfaceInfo>,
    pub total_download_rate: u64,
    pub total_upload_rate: u64,
    pub adapters: Vec<NetworkAdapterInfo>,
    pub adapter_status: Observation,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct InterfaceInfo {
    #[serde(default)]
    pub rate_status: Observation,
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

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct NetworkAdapterInfo {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub media_connection_state: Option<u32>,
    pub link_speed_bps: Option<u64>,
    pub hardware_interface: Option<bool>,
}

#[derive(Debug, Default)]
pub struct NetworkSampler {
    counters: HashMap<(String, String), (CounterRate, CounterRate)>,
    last_sample: Option<Instant>,
    sample: SampleMeta,
}

impl NetworkSampler {
    pub fn collect(&mut self, networks: &mut Networks) -> NetworkData {
        networks.refresh(true);
        let now = Instant::now();
        let interval = self
            .last_sample
            .replace(now)
            .map(|last| now.duration_since(last))
            .unwrap_or_default();
        let present: std::collections::HashSet<_> = networks
            .iter()
            .map(|(name, data)| (name.clone(), data.mac_address().to_string()))
            .collect();
        self.counters.retain(|key, _| present.contains(key));

        let interfaces: Vec<InterfaceInfo> = networks
            .iter()
            .map(|(name, data)| {
                let operational_state = data.operational_state().to_string();
                let normalized_state = operational_state.to_ascii_lowercase();
                let is_up = normalized_state == "up";
                let counters = self
                    .counters
                    .entry((name.clone(), data.mac_address().to_string()))
                    .or_default();
                let down = counters
                    .0
                    .sample(data.total_received(), now, Duration::from_secs(10));
                let up = counters
                    .1
                    .sample(data.total_transmitted(), now, Duration::from_secs(10));

                let ip_addresses: Vec<String> = data
                    .ip_networks()
                    .iter()
                    .map(|ip| ip.addr.to_string())
                    .collect();

                InterfaceInfo {
                    rate_status: if down.is_some() && up.is_some() {
                        Observation::available("interface counters / monotonic elapsed time")
                    } else {
                        Observation::unavailable(
                            "interface counters",
                            "Warming up after first sample, counter reset, or collection gap",
                        )
                    },
                    name: name.clone(),
                    ip_addresses,
                    mac_address: data.mac_address().to_string(),
                    received_bytes: data.total_received(),
                    transmitted_bytes: data.total_transmitted(),
                    download_rate: down.unwrap_or_default().round() as u64,
                    upload_rate: up.unwrap_or_default().round() as u64,
                    is_up,
                    operational_state,
                }
            })
            .collect();

        let total_download_rate = interfaces
            .iter()
            .fold(0u64, |sum, iface| sum.saturating_add(iface.download_rate));
        let total_upload_rate = interfaces
            .iter()
            .fold(0u64, |sum, iface| sum.saturating_add(iface.upload_rate));
        let observation = if !interfaces.is_empty()
            && interfaces
                .iter()
                .all(|iface| iface.rate_status.is_available())
        {
            Observation::available("interface counters / monotonic elapsed time")
        } else {
            Observation::unavailable(
                "interface counters",
                "One or more interfaces are warming up or unavailable",
            )
        };
        self.sample
            .record(interval, Duration::from_secs(1), observation);

        NetworkData {
            sample: self.sample.clone(),
            interfaces,
            total_download_rate,
            total_upload_rate,
            adapters: Vec::new(),
            adapter_status: Observation::default(),
        }
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
