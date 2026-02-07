use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use crate::types::{HealthStatus, TempUnit};

// -- Color Palette (QubeTX) --

pub const COLOR_GOOD: Color = Color::Green;
pub const COLOR_WARN: Color = Color::Yellow;
pub const COLOR_CRIT: Color = Color::Red;
pub const COLOR_INFO: Color = Color::Cyan;
pub const COLOR_ACCENT: Color = Color::Cyan;
pub const COLOR_DIM: Color = Color::DarkGray;
pub const COLOR_HEADER: Color = Color::Cyan;

pub fn status_color(status: &HealthStatus) -> Color {
    match status {
        HealthStatus::Good => COLOR_GOOD,
        HealthStatus::Warning => COLOR_WARN,
        HealthStatus::Critical => COLOR_CRIT,
        HealthStatus::Unknown => COLOR_DIM,
    }
}

// -- Formatters --

/// Format bytes to human-readable string (e.g., "12.4 GB")
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format bytes using GiB (binary) for technician mode
pub fn format_bytes_gib(bytes: u64) -> String {
    let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gib >= 1024.0 {
        format!("{:.1} TiB", gib / 1024.0)
    } else if gib >= 1.0 {
        format!("{:.2} GiB", gib)
    } else {
        let mib = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.1} MiB", mib)
    }
}

/// Format bytes per second as throughput
pub fn format_throughput(bytes_per_sec: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes_per_sec >= GB {
        format!("{:.1} GB/s", bytes_per_sec as f64 / GB as f64)
    } else if bytes_per_sec >= MB {
        format!("{:.1} MB/s", bytes_per_sec as f64 / MB as f64)
    } else if bytes_per_sec >= KB {
        format!("{:.1} KB/s", bytes_per_sec as f64 / KB as f64)
    } else {
        format!("{} B/s", bytes_per_sec)
    }
}

/// Format uptime seconds as "Xd Xh Xm"
pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// Plain language for percentage (User Mode)
pub fn plain_language_percent(pct: f64, resource: &str) -> String {
    if pct < 30.0 {
        format!("Plenty of {} available", resource)
    } else if pct < 60.0 {
        format!("Using a moderate amount of {}", resource)
    } else if pct < 80.0 {
        format!("Using most of the {}", resource)
    } else if pct < 95.0 {
        format!("{} is getting full", resource.to_string())
    } else {
        format!("{} is almost completely used", resource.to_string())
    }
}

/// Plain language for temperature (User Mode)
pub fn plain_language_temp(temp_c: f64) -> &'static str {
    if temp_c < 45.0 {
        "Cool"
    } else if temp_c < 65.0 {
        "Normal"
    } else if temp_c < 80.0 {
        "Warm"
    } else if temp_c < 95.0 {
        "Hot"
    } else {
        "Very hot"
    }
}

/// Format temperature value with unit
pub fn format_temp(temp_c: f64, unit: TempUnit) -> String {
    let value = unit.convert(temp_c);
    format!("{:.0}{}", value, unit.suffix())
}

/// Plain language for CPU load (User Mode)
pub fn plain_language_cpu(pct: f32) -> &'static str {
    if pct < 25.0 {
        "Running quietly"
    } else if pct < 50.0 {
        "Running normally"
    } else if pct < 75.0 {
        "Fairly busy right now"
    } else if pct < 90.0 {
        "Very busy"
    } else {
        "Extremely busy"
    }
}

/// Plain language for network speed
pub fn plain_language_speed(bytes_per_sec: u64) -> &'static str {
    const MB: u64 = 1024 * 1024;
    if bytes_per_sec < 100 * 1024 {
        "Slow"
    } else if bytes_per_sec < MB {
        "Moderate"
    } else if bytes_per_sec < 10 * MB {
        "Fast"
    } else {
        "Very fast"
    }
}

/// Create a text gauge bar like [████████░░] 78%
pub fn gauge_bar(percent: f64, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "[{}{}] {:.0}%",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
        percent
    )
}

/// Create a colored gauge line with label
pub fn gauge_line<'a>(label: &str, percent: f64, width: usize) -> Line<'a> {
    let status = HealthStatus::from_percent(percent);
    let color = status_color(&status);

    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    Line::from(vec![
        Span::styled(
            format!("  {:<14}", label),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!("[{}{}]", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty)),
            Style::default().fg(color),
        ),
        Span::styled(
            format!(" {:.0}%", percent),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Status icon + description line for User Mode
pub fn status_line<'a>(status: &HealthStatus, label: &str, description: &str) -> Line<'a> {
    let color = status_color(status);
    Line::from(vec![
        Span::styled(
            format!("  {} ", status.icon()),
            Style::default().fg(color),
        ),
        Span::styled(
            format!("{:<16}", label),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            description.to_string(),
            Style::default().fg(Color::DarkGray),
        ),
    ])
}

/// Section header line
pub fn section_header<'a>(title: &str) -> Line<'a> {
    Line::from(Span::styled(
        format!("  {}", title),
        Style::default()
            .fg(COLOR_HEADER)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Separator line
pub fn separator(width: usize) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", "\u{2500}".repeat(width.saturating_sub(4))),
        Style::default().fg(COLOR_DIM),
    ))
}
