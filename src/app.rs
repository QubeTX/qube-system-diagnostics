use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use std::time::Duration;
use tokio::time::interval;

use crate::collectors::SystemSnapshot;
use crate::error::Result;
use crate::history::HistoryBuffer;
use crate::monitor::{Lane, Monitor, Profile};
use crate::types::{DiagnosticMode, HealthStatus, ProcessSortKey, Section, TempUnit};
use crate::ui;

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
    pub show_findings: bool,
    /// Exceptional Cargo v2-to-v3 state: the second update still needs to
    /// install the managed CLI+GUI product. This never affects normal TUI
    /// startup or session behavior.
    pub cargo_gui_completion_notice: bool,
    /// System data snapshot
    pub snapshot: SystemSnapshot,
    pub findings: Vec<crate::findings::Finding>,
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
    /// Drivers section scroll offset (tech mode)
    pub driver_scroll: usize,
    /// Disk section scroll offset (tech mode)
    pub disk_scroll: usize,
    monitor: Option<Monitor>,
}

impl App {
    pub fn awaiting_initial_sample(&self) -> bool {
        self.monitor.is_some()
            && self
                .snapshot
                .samples
                .get("fast")
                .is_none_or(|s| s.sequence <= 1 && !s.observation.is_available())
    }

    pub fn new(initial_mode: Option<DiagnosticMode>) -> Self {
        Self {
            mode: initial_mode,
            current_section: Section::Overview,
            should_quit: false,
            show_help: false,
            show_findings: false,
            cargo_gui_completion_notice: false,
            snapshot: SystemSnapshot::default(),
            findings: Vec::new(),
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
            driver_scroll: 0,
            disk_scroll: 0,
            monitor: None,
        }
    }

    /// Render immediately. Workers own all collection; this loop only handles
    /// input and bounded latest-result delivery.
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.monitor = Some(Monitor::start(Profile::Full));
        let mut poll = interval(Duration::from_millis(50));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut events = crossterm::event::EventStream::new();
        let mut dirty = true;
        loop {
            if dirty {
                let size = terminal.size()?;
                self.too_small = size.width < 80 || size.height < 24;
                terminal.draw(|frame| ui::render(frame, self))?;
                dirty = false;
            }
            if self.should_quit {
                self.monitor.take();
                return Ok(());
            }
            tokio::select! {
                _ = poll.tick() => {
                    let changed = self.monitor.as_ref().map(|m| m.drain(&mut self.snapshot)).unwrap_or_default();
                    if changed.contains(&Lane::Fast) { self.update_fast_history(); }
                    if changed.contains(&Lane::Slow) { self.update_slow_history(); }
                    if changed.contains(&Lane::Activity) { self.update_activity_history(); }
                    if !changed.is_empty() { self.findings = crate::findings::for_snapshot(&self.snapshot); }
                    dirty |= !changed.is_empty();
                }
                event = events.next() => {
                    match event {
                        Some(Ok(event)) => { self.handle_event(event); dirty = true; }
                        Some(Err(error)) => return Err(error.into()),
                        None => { self.should_quit = true; }
                    }
                }
            }
        }
    }
    fn update_fast_history(&mut self) {
        let Some(sample) = self.snapshot.samples.get("fast") else {
            return;
        };
        let captured = sample.captured_unix_ms;
        if captured == 0
            || self
                .cpu_history
                .samples()
                .last()
                .is_some_and(|v| v.captured_unix_ms == captured)
        {
            return;
        }
        let cpu_available = sample.observation.is_available() && sample.interval_ms <= 10_000;
        self.cpu_history.push_at(
            captured,
            cpu_available.then_some(self.snapshot.cpu.total_usage as f64),
        );
        self.per_core_history
            .resize_with(self.snapshot.cpu.per_core_usage.len(), || {
                HistoryBuffer::new(HISTORY_SAMPLES)
            });
        for (buffer, usage) in self
            .per_core_history
            .iter_mut()
            .zip(&self.snapshot.cpu.per_core_usage)
        {
            buffer.push_at(captured, cpu_available.then_some(*usage as f64));
        }
        self.mem_history.push_at(
            captured,
            (self.snapshot.memory.total_bytes > 0).then(|| self.snapshot.memory.usage_percent()),
        );
        self.swap_history.push_at(
            captured,
            (self.snapshot.memory.swap_total_bytes > 0)
                .then(|| self.snapshot.memory.swap_percent()),
        );
        let network_available = self.snapshot.network.sample.observation.is_available();
        self.net_down_history.push_at(
            captured,
            network_available.then_some(self.snapshot.network.total_download_rate as f64),
        );
        self.net_up_history.push_at(
            captured,
            network_available.then_some(self.snapshot.network.total_upload_rate as f64),
        );
    }
    fn update_slow_history(&mut self) {
        let Some(sample) = self.snapshot.samples.get("slow") else {
            return;
        };
        let captured = sample.captured_unix_ms;
        if captured == 0
            || self
                .gpu_history
                .samples()
                .last()
                .is_some_and(|v| v.captured_unix_ms == captured)
        {
            return;
        }
        self.gpu_history.push_at(
            captured,
            self.snapshot
                .gpu
                .telemetry_available
                .then_some(self.snapshot.gpu.utilization_percent as f64),
        );
        // CPU and GPU temperatures must never be spliced into one series.
        self.temp_history
            .push_at(captured, self.snapshot.thermals.cpu_temp);
    }
    fn update_activity_history(&mut self) {
        let Some(sample) = self.snapshot.samples.get("activity") else {
            return;
        };
        if sample.captured_unix_ms == 0
            || self
                .disk_read_history
                .samples()
                .last()
                .is_some_and(|s| s.captured_unix_ms == sample.captured_unix_ms)
        {
            return;
        }
        let totals = self.snapshot.disk_activity.totals();
        self.disk_read_history
            .push_at(sample.captured_unix_ms, totals.map(|t| t.0));
        self.disk_write_history
            .push_at(sample.captured_unix_ms, totals.map(|t| t.1));
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
            if self.show_findings {
                if matches!(key.code, KeyCode::Char('F') | KeyCode::Esc) {
                    self.show_findings = false;
                }
                return;
            }
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
                KeyCode::Char('F') => self.show_findings = true,
                KeyCode::Char(c @ '1'..='9') => {
                    if let Some(section) = Section::from_number(c as u8 - b'0') {
                        self.current_section = section;
                        self.process_scroll = 0;
                        self.connection_scroll = 0;
                        self.driver_scroll = 0;
                        self.disk_scroll = 0;
                    }
                }
                // Scrollable table controls
                KeyCode::Char('j') | KeyCode::Down => {
                    match self.current_section {
                        Section::Processes => {
                            let max = self.snapshot.processes.list.len().saturating_sub(1);
                            self.process_scroll = (self.process_scroll + 1).min(max);
                        }
                        Section::Network => {
                            let max = self
                                .snapshot
                                .network_diag
                                .active_connections
                                .len()
                                .saturating_sub(1);
                            self.connection_scroll = (self.connection_scroll + 1).min(max);
                        }
                        Section::Drivers => {
                            // Upper bound clamped in render; just increment here
                            self.driver_scroll = self.driver_scroll.saturating_add(1);
                        }
                        Section::Disk => {
                            self.disk_scroll = self.disk_scroll.saturating_add(1);
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => match self.current_section {
                    Section::Processes => {
                        self.process_scroll = self.process_scroll.saturating_sub(1);
                    }
                    Section::Network => {
                        self.connection_scroll = self.connection_scroll.saturating_sub(1);
                    }
                    Section::Drivers => {
                        self.driver_scroll = self.driver_scroll.saturating_sub(1);
                    }
                    Section::Disk => {
                        self.disk_scroll = self.disk_scroll.saturating_sub(1);
                    }
                    _ => {}
                },
                KeyCode::Char('c') if self.current_section == Section::Processes => {
                    self.process_sort = ProcessSortKey::Cpu;
                }
                KeyCode::Char('M') if self.current_section == Section::Processes => {
                    self.process_sort = ProcessSortKey::Memory;
                }
                KeyCode::Char('n') if self.current_section == Section::Processes => {
                    self.process_sort = ProcessSortKey::Name;
                }
                KeyCode::Char('p') if self.current_section == Section::Processes => {
                    self.process_sort = ProcessSortKey::Pid;
                }
                // Temperature unit toggle
                KeyCode::Char('f') => {
                    self.temp_unit = self.temp_unit.toggle();
                }
                // Manual refresh for drivers section (non-blocking)
                KeyCode::Char('r') => {
                    if let Some(monitor) = &self.monitor {
                        for lane in Lane::ALL {
                            monitor.retry(lane);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Get overall system health status
    pub fn overall_health(&self) -> HealthStatus {
        if self.findings.iter().any(|f| f.severity == "critical") {
            HealthStatus::Critical
        } else if self.findings.iter().any(|f| f.severity == "warning") {
            HealthStatus::Warning
        } else if !self.findings.is_empty() || self.snapshot.samples.is_empty() {
            HealthStatus::Unknown
        } else {
            HealthStatus::Good
        }
    }
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;

    use crate::collectors::network_diag::{ConnectionInfo, ConnectionState, Protocol};
    use crate::collectors::processes::ProcessInfo;
    use crossterm::event::{KeyEvent, KeyEventKind};

    fn press(app: &mut App, code: KeyCode) {
        app.handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }

    fn press_with_modifiers(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
        app.handle_event(Event::Key(KeyEvent::new(code, modifiers)));
    }

    fn process(pid: u32) -> ProcessInfo {
        ProcessInfo {
            pid,
            name: format!("process-{pid}"),
            friendly_name: format!("Process {pid}"),
            cpu_percent: 0.0,
            memory_bytes: 0,
            memory_percent: 0.0,
            status: "Run".into(),
            ..Default::default()
        }
    }

    fn connection(port: u16) -> ConnectionInfo {
        ConnectionInfo {
            protocol: Protocol::Tcp,
            local_addr: "127.0.0.1".into(),
            local_port: port,
            remote_addr: "127.0.0.1".into(),
            remote_port: port,
            state: ConnectionState::Established,
            pid: None,
            process_name: None,
        }
    }

    #[test]
    fn v2_refresh_cadence_history_and_startup_defaults_are_unchanged() {
        assert_eq!(Lane::Fast.cadence(), Duration::from_secs(1));
        assert_eq!(Lane::Connections.cadence(), Duration::from_secs(3));
        assert_eq!(Lane::Slow.cadence(), Duration::from_secs(5));
        assert_eq!(Lane::Diagnostics.cadence(), Duration::from_secs(15));
        assert_eq!(Lane::Health.cadence(), Duration::from_secs(60));
        assert_eq!(HISTORY_SAMPLES, 60);

        let mut app = App::new(None);
        assert_eq!(app.mode, None);
        assert_eq!(app.current_section, Section::Overview);
        assert!(!app.should_quit);
        assert!(!app.show_help);
        assert_eq!(app.process_sort, ProcessSortKey::Cpu);
        assert_eq!(app.temp_unit, TempUnit::Celsius);
        assert_eq!(app.process_scroll, 0);
        assert_eq!(app.connection_scroll, 0);
        assert_eq!(app.driver_scroll, 0);
        assert_eq!(app.disk_scroll, 0);
        assert!(app.monitor.is_none());

        for sample in 0..=HISTORY_SAMPLES {
            app.cpu_history.push(sample as f64);
        }
        assert_eq!(app.cpu_history.len(), HISTORY_SAMPLES);
        assert_eq!(app.cpu_history.as_slice().first(), Some(&1.0));
    }

    #[test]
    fn v2_mode_chooser_help_navigation_and_quit_keys_are_unchanged() {
        let mut user = App::new(None);
        press(&mut user, KeyCode::Char('1'));
        assert_eq!(user.mode, Some(DiagnosticMode::User));

        let mut technician = App::new(None);
        press(&mut technician, KeyCode::Char('2'));
        assert_eq!(technician.mode, Some(DiagnosticMode::Technician));

        press(&mut user, KeyCode::Char('?'));
        assert!(user.show_help);
        press(&mut user, KeyCode::Char('7'));
        assert!(user.show_help, "help overlay should consume unrelated keys");
        assert_eq!(user.current_section, Section::Overview);
        press(&mut user, KeyCode::Esc);
        assert!(!user.show_help);
        assert!(
            !user.should_quit,
            "Escape should close help before quitting"
        );

        user.process_scroll = 3;
        user.connection_scroll = 4;
        user.driver_scroll = 5;
        user.disk_scroll = 6;
        press(&mut user, KeyCode::Char('7'));
        assert_eq!(user.current_section, Section::Processes);
        assert_eq!(user.process_scroll, 0);
        assert_eq!(user.connection_scroll, 0);
        assert_eq!(user.driver_scroll, 0);
        assert_eq!(user.disk_scroll, 0);

        press(&mut user, KeyCode::Char('m'));
        assert_eq!(user.mode, None);

        let mut quit = App::new(None);
        press(&mut quit, KeyCode::Char('q'));
        assert!(quit.should_quit);

        let mut control_c = App::new(Some(DiagnosticMode::Technician));
        control_c.show_help = true;
        press_with_modifiers(&mut control_c, KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(
            control_c.should_quit,
            "Ctrl+C must take priority everywhere"
        );
    }

    #[test]
    fn v2_section_unit_sort_and_scroll_keybindings_are_unchanged() {
        let mut app = App::new(Some(DiagnosticMode::Technician));
        for (key, section) in [
            ('1', Section::Overview),
            ('2', Section::Cpu),
            ('3', Section::Memory),
            ('4', Section::Disk),
            ('5', Section::Gpu),
            ('6', Section::Network),
            ('7', Section::Processes),
            ('8', Section::Thermals),
            ('9', Section::Drivers),
        ] {
            press(&mut app, KeyCode::Char(key));
            assert_eq!(app.current_section, section);
        }

        assert_eq!(app.temp_unit, TempUnit::Celsius);
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(app.temp_unit, TempUnit::Fahrenheit);
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(app.temp_unit, TempUnit::Celsius);

        app.current_section = Section::Processes;
        for (key, sort) in [
            ('c', ProcessSortKey::Cpu),
            ('M', ProcessSortKey::Memory),
            ('p', ProcessSortKey::Pid),
            ('n', ProcessSortKey::Name),
        ] {
            press(&mut app, KeyCode::Char(key));
            assert_eq!(app.process_sort, sort);
        }

        app.snapshot.processes.list = vec![process(1), process(2), process(3)];
        for _ in 0..4 {
            press(&mut app, KeyCode::Down);
        }
        assert_eq!(app.process_scroll, 2, "process scroll should clamp to rows");
        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.process_scroll, 1);

        app.current_section = Section::Network;
        app.snapshot.network_diag.active_connections = vec![connection(1), connection(2)];
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.connection_scroll, 1);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.connection_scroll, 0);

        app.current_section = Section::Drivers;
        press(&mut app, KeyCode::Down);
        assert_eq!(app.driver_scroll, 1);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.driver_scroll, 0);

        app.current_section = Section::Disk;
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.disk_scroll, 1);
        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.disk_scroll, 0);
    }

    #[test]
    fn key_release_events_remain_ignored() {
        let mut app = App::new(Some(DiagnosticMode::User));
        app.handle_event(Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )));
        assert!(!app.should_quit);
    }
}
