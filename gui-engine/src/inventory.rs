//! Search complete captured inventories before projecting a bounded GUI page.
use super::*;
use collectors::drivers::{DeviceInfo, DriverScanStatus, ServiceInfo};
use collectors::network_diag::ConnectionInfo;

pub const CONNECTION_ROWS: usize = 20;
pub const DEVICE_ROWS: usize = 32;

#[derive(Default, Clone)]
pub struct Query {
    pub filter: String,
    pub offset: usize,
    pub attention_only: bool,
}

#[derive(Serialize)]
struct Connections<'a> {
    active_connections: Vec<&'a ConnectionInfo>,
    listening_count: usize,
    total_count: usize,
    matched_count: usize,
    page_offset: usize,
}
#[derive(Serialize)]
struct Devices<'a> {
    devices: Vec<&'a DeviceInfo>,
    services: &'a [ServiceInfo],
    service_total_count: usize,
    scan_status: &'a DriverScanStatus,
    total_count: usize,
    attention_count: usize,
    matched_count: usize,
    page_offset: usize,
}
fn page_offset(requested: usize, count: usize, rows: usize) -> usize {
    (requested / rows * rows).min(count.saturating_sub(1) / rows * rows)
}
fn connections<'a>(snapshot: &'a SystemSnapshot, query: &Query) -> Connections<'a> {
    let mut matching: Vec<_> = snapshot
        .network_diag
        .active_connections
        .iter()
        .filter(|row| {
            query.filter.is_empty()
                || format!(
                    "{} {} {} {} {} {} {} {}",
                    row.protocol,
                    row.local_addr,
                    row.local_port,
                    row.remote_addr,
                    row.remote_port,
                    row.state,
                    row.pid.map(|p| p.to_string()).unwrap_or_default(),
                    row.process_name.as_deref().unwrap_or("")
                )
                .to_lowercase()
                .contains(&query.filter)
        })
        .collect();
    matching.sort_by_key(|row| {
        (
            row.local_addr.as_str(),
            row.local_port,
            row.remote_addr.as_str(),
            row.remote_port,
            row.pid,
            matches!(row.protocol, collectors::network_diag::Protocol::Udp),
        )
    });
    let matched_count = matching.len();
    let page_offset = page_offset(query.offset, matched_count, CONNECTION_ROWS);
    Connections {
        active_connections: matching
            .into_iter()
            .skip(page_offset)
            .take(CONNECTION_ROWS)
            .collect(),
        listening_count: snapshot.network_diag.listening_ports.len(),
        total_count: snapshot.network_diag.active_connections.len(),
        matched_count,
        page_offset,
    }
}
fn devices<'a>(snapshot: &'a SystemSnapshot, query: &Query) -> Devices<'a> {
    let mut matching: Vec<_> = snapshot
        .drivers
        .devices()
        .filter(|row| {
            (!query.attention_only || row.status.requires_attention())
                && (query.filter.is_empty()
                    || format!(
                        "{} {} {} {} {} {}",
                        row.name,
                        row.category.label(),
                        row.status,
                        row.driver_version,
                        row.driver_date,
                        row.extra
                    )
                    .to_lowercase()
                    .contains(&query.filter))
        })
        .collect();
    matching.sort_by_key(|row| {
        (
            !row.status.requires_attention(),
            row.name.as_str(),
            row.category.label(),
            row.extra.as_str(),
        )
    });
    let matched_count = matching.len();
    let page_offset = page_offset(query.offset, matched_count, DEVICE_ROWS);
    Devices {
        devices: matching
            .into_iter()
            .skip(page_offset)
            .take(DEVICE_ROWS)
            .collect(),
        services: &snapshot.drivers.services[..snapshot.drivers.services.len().min(32)],
        service_total_count: snapshot.drivers.services.len(),
        scan_status: &snapshot.drivers.scan_status,
        total_count: snapshot.drivers.devices().count(),
        attention_count: snapshot.drivers.attention_devices().count(),
        matched_count,
        page_offset,
    }
}
pub fn publish(shared: &Shared, snapshot: &SystemSnapshot, index: usize) {
    let lane = if index == 0 { "connections" } else { "drivers" };
    let Some(sample) = snapshot.samples.get(lane) else {
        return;
    };
    let Ok(queries) = shared.inventory_queries.lock() else {
        return;
    };
    if index == 0 {
        publish_sample(
            shared,
            Topic::Medium,
            &connections(snapshot, &queries[index]),
            &snapshot.warnings,
            Some(sample),
        );
    } else {
        publish_sample(
            shared,
            Topic::Drivers,
            &devices(snapshot, &queries[index]),
            &snapshot.warnings,
            Some(sample),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_boundary_rejects_invalid_topics_and_unbounded_input() {
        let mut handle = ptr::null_mut();
        assert_eq!(sd300_engine_create(&mut handle), STATUS_OK);
        assert_eq!(
            sd300_engine_set_inventory_query(handle, 1, ptr::null(), 0, 0, 0),
            STATUS_INVALID_TOPIC
        );
        assert_eq!(
            sd300_engine_set_inventory_query(handle, 2, ptr::null(), 257, 0, 0),
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            sd300_engine_set_inventory_query(handle, 6, [255].as_ptr(), 1, 0, 0),
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            sd300_engine_set_inventory_query(handle, 6, b"\n".as_ptr(), 1, 0, 0),
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            sd300_engine_set_inventory_query(handle, 2, ptr::null(), 0, u32::MAX, 0),
            STATUS_OK
        );
        assert_eq!(
            sd300_engine_set_inventory_query(handle, 6, b"disk".as_ptr(), 4, 32, 1),
            STATUS_OK
        );
        sd300_engine_destroy(handle);
    }
    #[test]
    fn search_and_pages_reach_the_full_inventory_and_retain_capture_time() {
        let mut snapshot = SystemSnapshot::default();
        snapshot.network_diag.active_connections = (0..250)
            .map(|i| ConnectionInfo {
                protocol: collectors::network_diag::Protocol::Tcp,
                local_addr: "127.0.0.1".into(),
                local_port: i,
                remote_addr: "192.0.2.1".into(),
                remote_port: 443,
                state: collectors::network_diag::ConnectionState::Established,
                pid: Some(i.into()),
                process_name: Some(format!("process-{i}")),
            })
            .collect();
        snapshot.drivers.other = (0..250)
            .map(|i| DeviceInfo {
                name: format!("Device-{i:03}"),
                category: collectors::drivers::DeviceCategory::Other,
                status: if i == 249 {
                    collectors::drivers::DeviceStatus::Disabled
                } else {
                    collectors::drivers::DeviceStatus::Ok
                },
                driver_version: String::new(),
                driver_date: String::new(),
                extra: String::new(),
            })
            .collect();
        let query = Query {
            filter: "249".into(),
            ..Default::default()
        };
        assert_eq!(
            connections(&snapshot, &query).active_connections[0].pid,
            Some(249)
        );
        assert_eq!(devices(&snapshot, &query).devices[0].name, "Device-249");
        let query = Query {
            offset: usize::MAX,
            ..Default::default()
        };
        let page = connections(&snapshot, &query);
        assert_eq!(
            (
                page.page_offset,
                page.matched_count,
                page.active_connections.len()
            ),
            (240, 250, 10)
        );
        let page = devices(&snapshot, &query);
        assert_eq!(
            (page.page_offset, page.matched_count, page.devices.len()),
            (224, 250, 26)
        );
        assert_eq!(
            devices(
                &snapshot,
                &Query {
                    attention_only: true,
                    ..Default::default()
                }
            )
            .matched_count,
            1
        );
        let shared = Shared::default();
        snapshot.samples.insert(
            "connections".into(),
            collectors::sampling::SampleMeta {
                captured_unix_ms: 42,
                ..Default::default()
            },
        );
        publish(&shared, &snapshot, 0);
        shared.inventory_queries.lock().unwrap()[0].filter = "249".into();
        publish(&shared, &snapshot, 0);
        let topics = shared.topics.lock().unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&topics[Topic::Medium as usize].json).unwrap();
        assert_eq!(value["captured_unix_ms"], 42);
        assert_eq!(value["data"]["matched_count"], 1);
        assert_eq!(value["sequence"], 2);
    }
}
