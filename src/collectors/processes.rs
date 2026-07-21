use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use sysinfo::System;

use crate::types::ProcessSortKey;

#[cfg(target_os = "windows")]
#[path = "processes_windows.rs"]
mod windows_gui;
#[cfg(target_os = "windows")]
pub use windows_gui::GuiProcessSampler;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProcessData {
    pub list: Vec<ProcessInfo>,
    pub total_count: usize,
    pub total_threads: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub friendly_name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub memory_percent: f64,
    pub status: String,
}

/// Map binary names to user-friendly application names (allocated once)
static FRIENDLY_NAMES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert("chrome", "Google Chrome");
    map.insert("chrome.exe", "Google Chrome");
    map.insert("firefox", "Mozilla Firefox");
    map.insert("firefox.exe", "Mozilla Firefox");
    map.insert("msedge", "Microsoft Edge");
    map.insert("msedge.exe", "Microsoft Edge");
    map.insert("spotify", "Spotify");
    map.insert("spotify.exe", "Spotify");
    map.insert("discord", "Discord");
    map.insert("discord.exe", "Discord");
    map.insert("code", "Visual Studio Code");
    map.insert("code.exe", "Visual Studio Code");
    map.insert("teams", "Microsoft Teams");
    map.insert("teams.exe", "Microsoft Teams");
    map.insert("msteams", "Microsoft Teams");
    map.insert("msteams.exe", "Microsoft Teams");
    map.insert("slack", "Slack");
    map.insert("slack.exe", "Slack");
    map.insert("explorer.exe", "Windows Explorer");
    map.insert("Finder", "Finder");
    map.insert("WindowServer", "Window Server");
    map.insert("dwm.exe", "Desktop Window Manager");
    map.insert("MsMpEng.exe", "Windows Defender");
    map.insert("svchost.exe", "System Service");
    map.insert("csrss.exe", "System Runtime");
    map.insert("System", "System");
    map.insert("systemd", "System Manager");
    map.insert("Xorg", "Display Server");
    map.insert("gnome-shell", "GNOME Desktop");
    map.insert("plasmashell", "KDE Desktop");
    map.insert("iTerm2", "iTerm");
    map.insert("Terminal", "Terminal");
    map.insert("WindowsTerminal.exe", "Windows Terminal");
    map.insert("wt.exe", "Windows Terminal");
    map.insert("obs64.exe", "OBS Studio");
    map.insert("obs", "OBS Studio");
    map.insert("steam", "Steam");
    map.insert("steam.exe", "Steam");
    map.insert("vlc", "VLC Media Player");
    map.insert("vlc.exe", "VLC Media Player");
    map.insert("node", "Node.js");
    map.insert("node.exe", "Node.js");
    map.insert("python3", "Python");
    map.insert("python.exe", "Python");
    map.insert("java", "Java");
    map.insert("java.exe", "Java");
    map.insert("cursor.exe", "Cursor");
    map.insert("brave.exe", "Brave Browser");
    map.insert("brave", "Brave Browser");
    map.insert("arc", "Arc Browser");
    map.insert("safari", "Safari");
    map
});

fn get_friendly_name(process_name: &str) -> String {
    FRIENDLY_NAMES
        .get(process_name)
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Strip common extensions for display
            process_name
                .strip_suffix(".exe")
                .unwrap_or(process_name)
                .to_string()
        })
}

pub fn collect(sys: &System) -> ProcessData {
    let total_memory = sys.total_memory();
    let mut processes: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p| {
            let name = p.name().to_string_lossy().to_string();
            let friendly = get_friendly_name(&name);
            let mem = p.memory();
            let mem_pct = if total_memory > 0 {
                (mem as f64 / total_memory as f64) * 100.0
            } else {
                0.0
            };

            ProcessInfo {
                pid: p.pid().as_u32(),
                name,
                friendly_name: friendly,
                cpu_percent: p.cpu_usage(),
                memory_bytes: mem,
                memory_percent: mem_pct,
                status: format!("{:?}", p.status()),
            }
        })
        .collect();

    let total_count = processes.len();

    // Sort by CPU usage descending by default
    processes.sort_by(|a, b| {
        b.cpu_percent
            .partial_cmp(&a.cpu_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Keep top 100 for display
    processes.truncate(100);

    ProcessData {
        list: processes,
        total_count,
        total_threads: 0,
    }
}

fn is_ranked_consumer(pid: u32) -> bool {
    pid != 0
}

#[cfg(any(target_os = "windows", test))]
pub(super) fn sort_process_info_rows(rows: &mut [ProcessInfo], sort: ProcessSortKey) {
    match sort {
        ProcessSortKey::Cpu => rows.sort_by(|a, b| {
            b.cpu_percent
                .total_cmp(&a.cpu_percent)
                .then_with(|| a.pid.cmp(&b.pid))
        }),
        ProcessSortKey::Memory => rows.sort_by(|a, b| {
            b.memory_bytes
                .cmp(&a.memory_bytes)
                .then_with(|| a.pid.cmp(&b.pid))
        }),
        ProcessSortKey::Pid => rows.sort_by_key(|process| process.pid),
        ProcessSortKey::Name => rows.sort_by_cached_key(|process| {
            (
                process.friendly_name.to_ascii_lowercase(),
                process.name.to_ascii_lowercase(),
                process.pid,
            )
        }),
    }
}

/// Build the GUI's bounded process projection without allocating names and
/// status strings for every process on the machine. The TUI keeps using
/// `collect` above and retains its top-100 contract.
pub fn collect_limited(sys: &System, limit: usize, sort: ProcessSortKey) -> ProcessData {
    let total_memory = sys.total_memory();
    let total_count = sys.processes().len();
    // PID 0 is an operating-system idle accounting row on Windows, not a
    // process consuming capacity. Keep it in the total inventory count but
    // never rank it among actionable GUI consumers.
    let mut ranked = sys
        .processes()
        .values()
        .filter(|process| is_ranked_consumer(process.pid().as_u32()))
        .collect::<Vec<_>>();
    match sort {
        ProcessSortKey::Cpu => ranked.sort_by(|a, b| {
            b.cpu_usage()
                .total_cmp(&a.cpu_usage())
                .then_with(|| a.pid().as_u32().cmp(&b.pid().as_u32()))
        }),
        ProcessSortKey::Memory => ranked.sort_by(|a, b| {
            b.memory()
                .cmp(&a.memory())
                .then_with(|| a.pid().as_u32().cmp(&b.pid().as_u32()))
        }),
        ProcessSortKey::Pid => ranked.sort_by_key(|process| process.pid().as_u32()),
        ProcessSortKey::Name => ranked.sort_by_cached_key(|process| {
            let name = process.name().to_string_lossy();
            (
                get_friendly_name(&name).to_ascii_lowercase(),
                name.to_ascii_lowercase(),
                process.pid().as_u32(),
            )
        }),
    }
    ranked.truncate(limit);

    let list = ranked
        .into_iter()
        .map(|process| {
            let name = process.name().to_string_lossy().to_string();
            let memory_bytes = process.memory();
            ProcessInfo {
                pid: process.pid().as_u32(),
                friendly_name: get_friendly_name(&name),
                name,
                cpu_percent: process.cpu_usage(),
                memory_bytes,
                memory_percent: if total_memory > 0 {
                    (memory_bytes as f64 / total_memory as f64) * 100.0
                } else {
                    0.0
                },
                status: format!("{:?}", process.status()),
            }
        })
        .collect();

    ProcessData {
        list,
        total_count,
        total_threads: 0,
    }
}

#[cfg(test)]
mod gui_projection_tests {
    use super::*;

    fn row(pid: u32, name: &str, friendly_name: &str, cpu: f32, memory: u64) -> ProcessInfo {
        ProcessInfo {
            pid,
            name: name.into(),
            friendly_name: friendly_name.into(),
            cpu_percent: cpu,
            memory_bytes: memory,
            memory_percent: 0.0,
            status: "Run".into(),
        }
    }

    fn fixture() -> Vec<ProcessInfo> {
        vec![
            row(40, "zulu.exe", "Zulu", 20.0, 100),
            row(20, "alpha-b.exe", "Alpha", 5.0, 900),
            row(10, "alpha-a.exe", "Alpha", 20.0, 500),
        ]
    }

    #[test]
    fn gui_rows_support_all_full_inventory_sort_keys_with_stable_pid_ties() {
        let mut rows = fixture();
        sort_process_info_rows(&mut rows, ProcessSortKey::Cpu);
        assert_eq!(
            rows.iter().map(|row| row.pid).collect::<Vec<_>>(),
            [10, 40, 20]
        );

        let mut rows = fixture();
        sort_process_info_rows(&mut rows, ProcessSortKey::Memory);
        assert_eq!(
            rows.iter().map(|row| row.pid).collect::<Vec<_>>(),
            [20, 10, 40]
        );

        let mut rows = fixture();
        sort_process_info_rows(&mut rows, ProcessSortKey::Pid);
        assert_eq!(
            rows.iter().map(|row| row.pid).collect::<Vec<_>>(),
            [10, 20, 40]
        );

        let mut rows = fixture();
        sort_process_info_rows(&mut rows, ProcessSortKey::Name);
        assert_eq!(
            rows.iter().map(|row| row.pid).collect::<Vec<_>>(),
            [10, 20, 40]
        );
    }

    #[test]
    fn pid_zero_is_inventory_only_and_never_an_actionable_consumer() {
        assert!(!is_ranked_consumer(0));
        assert!(is_ranked_consumer(4));
    }
}
