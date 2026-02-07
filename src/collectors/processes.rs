use sysinfo::System;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Default)]
pub struct ProcessData {
    pub list: Vec<ProcessInfo>,
    pub total_count: usize,
    pub total_threads: usize,
}

#[derive(Debug, Clone)]
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
    FRIENDLY_NAMES.get(process_name)
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
    processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));

    // Keep top 100 for display
    processes.truncate(100);

    ProcessData {
        list: processes,
        total_count,
        total_threads: 0,
    }
}
