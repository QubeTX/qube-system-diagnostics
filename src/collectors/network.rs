use sysinfo::{Networks, System};

#[derive(Debug, Clone, Default)]
pub struct NetworkData {
    pub interfaces: Vec<InterfaceInfo>,
    pub total_download_rate: u64,
    pub total_upload_rate: u64,
    /// Previous bytes for rate calculation
    prev_total_rx: u64,
    prev_total_tx: u64,
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
}

pub fn collect(_sys: &System, previous: &NetworkData) -> NetworkData {
    let networks = Networks::new_with_refreshed_list();

    let mut total_rx: u64 = 0;
    let mut total_tx: u64 = 0;

    let interfaces: Vec<InterfaceInfo> = networks
        .iter()
        .map(|(name, data)| {
            let rx = data.total_received();
            let tx = data.total_transmitted();
            total_rx += rx;
            total_tx += tx;

            let ip_addresses: Vec<String> = data
                .ip_networks()
                .iter()
                .map(|ip| ip.addr.to_string())
                .collect();

            InterfaceInfo {
                name: name.clone(),
                ip_addresses,
                mac_address: data.mac_address().to_string(),
                received_bytes: rx,
                transmitted_bytes: tx,
                download_rate: data.received(),
                upload_rate: data.transmitted(),
                is_up: data.total_received() > 0 || data.total_transmitted() > 0,
            }
        })
        .collect();

    // Calculate total rates (bytes since last refresh)
    let total_download_rate = if previous.prev_total_rx > 0 {
        total_rx.saturating_sub(previous.prev_total_rx)
    } else {
        0
    };
    let total_upload_rate = if previous.prev_total_tx > 0 {
        total_tx.saturating_sub(previous.prev_total_tx)
    } else {
        0
    };

    NetworkData {
        interfaces,
        total_download_rate,
        total_upload_rate,
        prev_total_rx: total_rx,
        prev_total_tx: total_tx,
    }
}
