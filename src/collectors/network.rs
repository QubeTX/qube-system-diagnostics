use sysinfo::Networks;

#[derive(Debug, Clone, Default)]
pub struct NetworkData {
    pub interfaces: Vec<InterfaceInfo>,
    pub total_download_rate: u64,
    pub total_upload_rate: u64,
}

#[derive(Debug, Clone)]
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
    }
}
