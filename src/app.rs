use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use std::time::Duration;
use tokio::time::interval;

use crate::collectors::drivers::{DriverData, DriverScanStatus};
use crate::collectors::{DiagnosticWarning, WarningSeverity, SystemSnapshot};
use crate::error::Result;
use crate::history::HistoryBuffer;
use crate::types::{DiagnosticMode, HealthStatus, ProcessSortKey, Section, TempUnit};
use crate::ui;

// -- Refresh Intervals --
const REFRESH_FAST: Duration = Duration::from_secs(1);
const REFRESH_SLOW: Duration = Duration::from_secs(5);
const REFRESH_MEDIUM: Duration = Duration::from_secs(3);
const REFRESH_DIAG: Duration = Duration::from_secs(15);
const REFRESH_HEALTH: Duration = Duration::from_secs(60);
const HISTORY_SAMPLES: usize = 60;

/// Main application state
pub struct App {
    /// Current diagnostic mode (None = show mode selection screen)
    pub mode: Option<DiagnosticMode>,
    /// Currently active section
    pub current_section: Section,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Whether to show the help overlay
    pub show_help: bool,
    /// System data snapshot
    pub snapshot: SystemSnapshot,
    /// CPU usage history (60 samples)
    pub cpu_history: HistoryBuffer,
    /// Memory usage history
    pub mem_history: HistoryBuffer,
    /// Network download history
    pub net_down_history: HistoryBuffer,
    /// Network upload history
    pub net_up_history: HistoryBuffer,
    /// Process table scroll offset
    pub process_scroll: usize,
    /// Process sort key
    pub process_sort: ProcessSortKey,
    /// Whether terminal is too small
    pub too_small: bool,
    /// Per-core CPU history
    pub per_core_history: Vec<HistoryBuffer>,
    /// Swap usage history
    pub swap_history: HistoryBuffer,
    /// GPU usage history
    pub gpu_history: HistoryBuffer,
    /// Temperature history
    pub temp_history: HistoryBuffer,
    /// Temperature display unit (Celsius or Fahrenheit)
    pub temp_unit: TempUnit,
    /// Network connection table scroll offset
    pub connection_scroll: usize,
    /// Disk I/O read history
    pub disk_read_history: HistoryBuffer,
    /// Disk I/O write history
    pub disk_write_history: HistoryBuffer,
    /// Async driver scan handle
    driver_scan_handle: Option<tokio::task::JoinHandle<DriverData>>,
}

impl App {
    pub fn new(initial_mode: Option<DiagnosticMode>) -> Self {
        Self {
            mode: initial_mode,
            current_section: Section::Overview,
            should_quit: false,
            show_help: false,
            snapshot: SystemSnapshot::default(),
            cpu_history: HistoryBuffer::new(HISTORY_SAMPLES),
            mem_history: HistoryBuffer::new(HISTORY_SAMPLES),
            net_down_history: HistoryBuffer::new(HISTORY_SAMPLES),
            net_up_history: HistoryBuffer::new(HISTORY_SAMPLES),
            process_scroll: 0,
            process_sort: ProcessSortKey::Cpu,
            too_small: false,
            per_core_history: Vec::new(),
            swap_history: HistoryBuffer::new(HISTORY_SAMPLES),
            gpu_history: HistoryBuffer::new(HISTORY_SAMPLES),
            temp_history: HistoryBuffer::new(HISTORY_SAMPLES),
            temp_unit: TempUnit::Celsius,
            connection_scroll: 0,
            disk_read_history: HistoryBuffer::new(HISTORY_SAMPLES),
            disk_write_history: HistoryBuffer::new(HISTORY_SAMPLES),
            driver_scan_handle: None,
        }
    }

    /// Run the main event loop
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        // Initial data collection
        self.snapshot.refresh_static();
        self.snapshot.refresh_fast();
        self.snapshot.refresh_slow();
        self.snapshot.refresh_connections();
        self.snapshot.refresh_disk_health();

        // Initial driver scan — async to avoid blocking UI
        self.snapshot.drivers.scan_status = DriverScanStatus::Scanning;
        self.driver_scan_handle = Some(tokio::task::spawn_blocking(|| {
            crate::collectors::drivers::collect()
        }));

        // Initial connectivity check in background
        {
            let gateway = crate::collectors::network_diag::collect_connectivity();
            self.snapshot.network_diag.gateway = gateway.0.gateway;
            self.snapshot.network_diag.dns = gateway.0.dns;
            self.snapshot.network_diag.internet = gateway.0.internet;
            self.snapshot.warnings.extend(gateway.1);
        }

        let mut fast_tick = interval(REFRESH_FAST);
        let mut slow_tick = interval(REFRESH_SLOW);
        let mut medium_tick = interval(REFRESH_MEDIUM);
        let mut diag_tick = interval(REFRESH_DIAG);
        let mut health_tick = interval(REFRESH_HEALTH);
        let mut event_stream = crossterm::event::EventStream::new();

        loop {
            // Check if async driver scan completed
            if let Some(ref handle) = self.driver_scan_handle {
                if handle.is_finished() {
                    if let Some(handle) = self.driver_scan_handle.take() {
                        if let Ok(data) = handle.await {
                            self.snapshot.warnings.retain(|w| w.source != "Drivers");
                            if let DriverScanStatus::WmiUnavailable(ref msg) = data.scan_status {
                                self.snapshot.warnings.push(DiagnosticWarning {
                                    source: "Drivers".into(),
                                    message: msg.clone(),
                                    severity: WarningSeverity::Warning,
                                });
                            }
                            self.snapshot.drivers = data;
                        }
                    }
                }
            }

            // Draw
            let size = terminal.size()?;
            self.too_small = size.width < 80 || size.height < 24;
            terminal.draw(|frame| ui::render(frame, self))?;

            if self.should_quit {
                return Ok(());
            }

            // Event handling with tokio select
            tokio::select! {
                _ = fast_tick.tick() => {
                    self.snapshot.refresh_fast();
                    self.update_fast_history();
                }
                _ = slow_tick.tick() => {
                    self.snapshot.refresh_slow();
                }
                _ = medium_tick.tick() => {
                    self.snapshot.refresh_connections();
                }
                _ = diag_tick.tick() => {
                    // Run connectivity checks in a blocking task to avoid UI freeze
                    let (diag_data, diag_warnings) = tokio::task::spawn_blocking(|| {
                        crate::collectors::network_diag::collect_connectivity()
                    }).await.unwrap_or_default();
                    self.snapshot.network_diag.gateway = diag_data.gateway;
                    self.snapshot.network_diag.dns = diag_data.dns;
                    self.snapshot.network_diag.internet = diag_data.internet;
                    self.snapshot.warnings.retain(|w| w.source != "Network");
                    self.snapshot.warnings.extend(diag_warnings);
                }
                _ = health_tick.tick() => {
                    self.snapshot.refresh_disk_health();
                }
                event = event_stream.next() => {
                    if let Some(Ok(evt)) = event {
                        self.handle_event(evt);
                    }
                }
            }
        }
    }

    fn update_fast_history(&mut self) {
        // CPU total
        self.cpu_history.push(self.snapshot.cpu.total_usage as f64);

        // Per-core
        while self.per_core_history.len() < self.snapshot.cpu.per_core_usage.len() {
            self.per_core_history.push(HistoryBuffer::new(HISTORY_SAMPLES));
        }
        for (i, usage) in self.snapshot.cpu.per_core_usage.iter().enumerate() {
            if let Some(buf) = self.per_core_history.get_mut(i) {
                buf.push(*usage as f64);
            }
        }

        // Memory
        let mem_pct = if self.snapshot.memory.total_bytes > 0 {
            (self.snapshot.memory.used_bytes as f64 / self.snapshot.memory.total_bytes as f64) * 100.0
        } else {
            0.0
        };
        self.mem_history.push(mem_pct);

        // Swap
        let swap_pct = if self.snapshot.memory.swap_total_bytes > 0 {
            (self.snapshot.memory.swap_used_bytes as f64 / self.snapshot.memory.swap_total_bytes as f64) * 100.0
        } else {
            0.0
        };
        self.swap_history.push(swap_pct);

        // Network
        self.net_down_history.push(self.snapshot.network.total_download_rate as f64);
        self.net_up_history.push(self.snapshot.network.total_upload_rate as f64);

        // GPU
        self.gpu_history.push(self.snapshot.gpu.utilization_percent as f64);

        // Temperature
        if let Some(cpu_temp) = self.snapshot.thermals.cpu_temp {
            self.temp_history.push(cpu_temp);
        }

        // Disk I/O
        if let Some(drive) = self.snapshot.disk_health.drives.first() {
            if let Some(ref io) = drive.io_stats {
                self.disk_read_history.push(io.read_bytes_per_sec as f64);
                self.disk_write_history.push(io.write_bytes_per_sec as f64);
            }
        }
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return;
            }

            // Ctrl+C always quits immediately
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.should_quit = true;
                return;
            }

            // Help overlay takes priority
            if self.show_help {
                match key.code {
                    KeyCode::Char('?') | KeyCode::Esc => self.show_help = false,
                    _ => {}
                }
                return;
            }

            // Mode selection screen
            if self.mode.is_none() {
                match key.code {
                    KeyCode::Char('1') => self.mode = Some(DiagnosticMode::User),
                    KeyCode::Char('2') => self.mode = Some(DiagnosticMode::Technician),
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    _ => {}
                }
                return;
            }

            // Main navigation
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Char('m') => self.mode = None,
                KeyCode::Char('?') => self.show_help = true,
                KeyCode::Char(c @ '1'..='9') => {
                    if let Some(section) = Section::from_number(c as u8 - b'0') {
                        self.current_section = section;
                        self.process_scroll = 0;
                        self.connection_scroll = 0;
                    }
                }
                // Scrollable table controls
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.current_section == Section::Processes {
                        let max = self.snapshot.processes.list.len().saturating_sub(1);
                        self.process_scroll = (self.process_scroll + 1).min(max);
                    } else if self.current_section == Section::Network {
                        let max = self.snapshot.network_diag.active_connections.len().saturating_sub(1);
                        self.connection_scroll = (self.connection_scroll + 1).min(max);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.current_section == Section::Processes {
                        self.process_scroll = self.process_scroll.saturating_sub(1);
                    } else if self.current_section == Section::Network {
                        self.connection_scroll = self.connection_scroll.saturating_sub(1);
                    }
                }
                KeyCode::Char('c') => {
                    if self.current_section == Section::Processes {
                        self.process_sort = ProcessSortKey::Cpu;
                    }
                }
                KeyCode::Char('n') => {
                    if self.current_section == Section::Processes {
                        self.process_sort = ProcessSortKey::Name;
                    }
                }
                KeyCode::Char('p') => {
                    if self.current_section == Section::Processes {
                        self.process_sort = ProcessSortKey::Pid;
                    }
                }
                // Temperature unit toggle
                KeyCode::Char('f') => {
                    self.temp_unit = self.temp_unit.toggle();
                }
                // Manual refresh for drivers section (non-blocking)
                KeyCode::Char('r') => {
                    if self.current_section == Section::Drivers && self.driver_scan_handle.is_none() {
                        self.snapshot.drivers.scan_status = DriverScanStatus::Scanning;
                        self.driver_scan_handle = Some(tokio::task::spawn_blocking(|| {
                            crate::collectors::drivers::collect()
                        }));
                    }
                }
                _ => {}
            }
        }
    }

    /// Get overall system health status
    pub fn overall_health(&self) -> HealthStatus {
        let cpu_status = HealthStatus::from_percent(self.snapshot.cpu.total_usage as f64);
        let mem_pct = if self.snapshot.memory.total_bytes > 0 {
            (self.snapshot.memory.used_bytes as f64 / self.snapshot.memory.total_bytes as f64) * 100.0
        } else {
            0.0
        };
        let mem_status = HealthStatus::from_percent(mem_pct);

        // Worst of all statuses
        if cpu_status == HealthStatus::Critical || mem_status == HealthStatus::Critical {
            HealthStatus::Critical
        } else if cpu_status == HealthStatus::Warning || mem_status == HealthStatus::Warning {
            HealthStatus::Warning
        } else {
            HealthStatus::Good
        }
    }
}
