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
    #[serde(default)]
    pub aggregation: String,
}

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct InterfaceInfo {
    #[serde(default)]
    pub rate_status: Observation,
    #[serde(default)]
    pub address_status: Observation,
    #[serde(default)]
    pub included_in_total: bool,
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
    counters: HashMap<String, (CounterRate, CounterRate)>,
    last_sample: Option<Instant>,
    sample: SampleMeta,
}

impl NetworkSampler {
    pub fn collect(&mut self, networks: &mut Networks) -> NetworkData {
        #[cfg(windows)]
        let (rows, source, aggregation) = {
            let _ = networks;
            (
                super::windows_network::collect(),
                super::windows_network::SOURCE,
                super::windows_network::AGGREGATION,
            )
        };
        #[cfg(not(windows))]
        let (rows, source, aggregation) = {
            networks.refresh(true);
            let rows = networks
                .iter()
                .map(|(name, data)| {
                    let mac = data.mac_address().to_string();
                    let operational_state = data.operational_state().to_string();
                    (
                        format!("{name}:{mac}"),
                        InterfaceInfo {
                            name: name.clone(),
                            ip_addresses: data
                                .ip_networks()
                                .iter()
                                .map(|ip| ip.addr.to_string())
                                .collect(),
                            mac_address: mac,
                            received_bytes: data.total_received(),
                            transmitted_bytes: data.total_transmitted(),
                            is_up: operational_state.eq_ignore_ascii_case("up"),
                            operational_state,
                            included_in_total: true,
                            address_status: Observation::available("platform interface addresses"),
                            ..Default::default()
                        },
                    )
                })
                .collect();
            (Ok(rows), "interface counters / monotonic elapsed time", "Sum of reported interface rates; traffic traversing multiple interfaces may appear more than once")
        };
        self.sample(rows, source, aggregation, Instant::now())
    }
    fn sample(
        &mut self,
        rows: Result<Vec<(String, InterfaceInfo)>, Observation>,
        source: &str,
        aggregation: &str,
        now: Instant,
    ) -> NetworkData {
        let mut rows = match rows {
            Ok(rows) => rows,
            Err(observation) => {
                // Do not stamp cached counter values as a new successful capture.
                // Recovery requires a new baseline; a failed read is never zero traffic.
                self.counters.clear();
                self.last_sample = None;
                self.sample.observation = observation;
                return NetworkData {
                    sample: self.sample.clone(),
                    aggregation: aggregation.into(),
                    ..Default::default()
                };
            }
        };
        let interval = self
            .last_sample
            .replace(now)
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default();
        let present: std::collections::HashSet<_> =
            rows.iter().map(|(id, _)| id.as_str()).collect();
        self.counters
            .retain(|key, _| present.contains(key.as_str()));
        for (identity, info) in &mut rows {
            let counters = self.counters.entry(identity.clone()).or_default();
            let down = counters
                .0
                .sample(info.received_bytes, now, Duration::from_secs(10));
            let up = counters
                .1
                .sample(info.transmitted_bytes, now, Duration::from_secs(10));
            info.rate_status = if down.is_some() && up.is_some() {
                Observation::available(source)
            } else {
                Observation::unavailable(
                    source,
                    "Warming up after first sample, counter reset, or collection gap",
                )
            };
            info.download_rate = down.unwrap_or_default().round() as u64;
            info.upload_rate = up.unwrap_or_default().round() as u64;
        }
        rows.sort_by(|a, b| a.1.name.cmp(&b.1.name).then_with(|| a.0.cmp(&b.0)));
        let interfaces: Vec<_> = rows.into_iter().map(|(_, info)| info).collect();
        let included: Vec<_> = interfaces.iter().filter(|i| i.included_in_total).collect();
        let total_download_rate = included
            .iter()
            .fold(0u64, |sum, iface| sum.saturating_add(iface.download_rate));
        let total_upload_rate = included
            .iter()
            .fold(0u64, |sum, iface| sum.saturating_add(iface.upload_rate));
        let observation = if !included.is_empty()
            && included.iter().all(|i| i.rate_status.is_available())
        {
            Observation::available(source)
        } else {
            Observation::unavailable(
                source,
                if included.is_empty() {
                    "No interfaces are eligible for this aggregate; inspect individual interfaces"
                } else {
                    "One or more included interfaces are warming up or unavailable"
                },
            )
        };
        self.sample
            .record(interval, Duration::from_secs(1), observation);
        NetworkData {
            sample: self.sample.clone(),
            interfaces,
            total_download_rate,
            total_upload_rate,
            aggregation: aggregation.into(),
            ..Default::default()
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

    #[cfg(target_os = "linux")]
    {
        let (adapters, status) =
            super::linux_inventory::adapters(std::path::Path::new("/sys/class/net"));
        data.adapters = adapters;
        data.adapter_status = status;
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        data.adapter_status = Observation::unsupported(
            "platform network adapter provider",
            "Static adapter capabilities are not implemented on this platform",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows(id: &str, count: u64) -> Vec<(String, InterfaceInfo)> {
        vec![(
            id.into(),
            InterfaceInfo {
                name: "identical alias".into(),
                received_bytes: count,
                transmitted_bytes: count,
                included_in_total: true,
                address_status: Observation::available("fixture"),
                ..Default::default()
            },
        )]
    }
    #[test]
    fn irregular_intervals_identity_replacement_and_failures_preserve_truth() {
        let mut sampler = NetworkSampler::default();
        let start = Instant::now();
        let first = sampler.sample(Ok(rows("one", 100)), "fixture", "fixture", start);
        assert!(!first.interfaces[0].rate_status.is_available());
        let second = sampler.sample(
            Ok(rows("one", 1100)),
            "fixture",
            "fixture",
            start + Duration::from_millis(2500),
        );
        assert_eq!(second.total_download_rate, 400);
        assert!(second.sample.observation.is_available());
        let failure = sampler.sample(
            Err(Observation::permission_denied("fixture", "denied")),
            "fixture",
            "fixture",
            start + Duration::from_secs(3),
        );
        assert_eq!(failure.sample.sequence, second.sample.sequence);
        assert_eq!(
            failure.sample.captured_unix_ms,
            second.sample.captured_unix_ms
        );
        assert!(failure.interfaces.is_empty());
        assert_eq!(
            failure.sample.observation.status,
            crate::observation::ObservationStatus::PermissionDenied
        );
        let recovery = sampler.sample(
            Ok(rows("one", 1500)),
            "fixture",
            "fixture",
            start + Duration::from_secs(4),
        );
        assert!(!recovery.interfaces[0].rate_status.is_available());
        let mut mixed = rows("one", 1700);
        let mut virtual_interface = rows("virtual", 1000).remove(0);
        virtual_interface.1.included_in_total = false;
        mixed.push(virtual_interface);
        let mixed = sampler.sample(
            Ok(mixed),
            "fixture",
            "fixture",
            start + Duration::from_secs(5),
        );
        assert_eq!(mixed.interfaces.len(), 2);
        assert_eq!(mixed.total_download_rate, 200);
        assert!(mixed.sample.observation.is_available()); // warming virtual row is excluded
        let replaced = sampler.sample(
            Ok(rows("two", 5000)),
            "fixture",
            "fixture",
            start + Duration::from_secs(6),
        );
        assert!(!replaced.interfaces[0].rate_status.is_available());
        assert_eq!(sampler.counters.len(), 1);
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
