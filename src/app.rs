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
    pub show_companion: bool,
    pub companion: crate::companion::Controller,
    pub optional_setup: crate::optional_tools::Controller,
    pub storage_probe: crate::storage_probe::Controller,
    pub show_storage_probe: bool,
    pub exporter: crate::export::Controller,
    pub show_export: bool,
    pub setup_confirmation: bool,
    pub setup_smart: bool,
    pub speed_confirmation: Option<crate::companion::Action>,
    pub mlab_consent: bool,
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
    pub gpu_temp_history: HistoryBuffer,
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
    pub view: crate::presentation::Presentation,
    pub filter: String,
    pub editing_filter: bool,
    pub show_inspector: bool,
    pub inspector_scroll: u16,
    pub sort_reversed: bool,
    pub paused_at: Option<u64>,
    pub presentation_time_ms: u64,
    pub preferences: crate::settings::TuiSettings,
    pub terminal_width: u16,
    pub terminal_height: u16,
    live_while_paused: Option<SystemSnapshot>,
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
            show_companion: false,
            companion: Default::default(),
            optional_setup: Default::default(),
            storage_probe: Default::default(),
            show_storage_probe: false,
            exporter: Default::default(),
            show_export: false,
            setup_confirmation: false,
            setup_smart: false,
            speed_confirmation: None,
            mlab_consent: false,
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
            gpu_temp_history: HistoryBuffer::new(HISTORY_SAMPLES),
            temp_unit: TempUnit::Celsius,
            connection_scroll: 0,
            disk_read_history: HistoryBuffer::new(HISTORY_SAMPLES),
            disk_write_history: HistoryBuffer::new(HISTORY_SAMPLES),
            driver_scroll: 0,
            disk_scroll: 0,
            view: Default::default(),
            filter: String::new(),
            editing_filter: false,
            show_inspector: false,
            inspector_scroll: 0,
            sort_reversed: false,
            paused_at: None,
            presentation_time_ms: crate::collectors::sampling::unix_ms(),
            preferences: Default::default(),
            terminal_width: 80,
            terminal_height: 24,
            live_while_paused: None,
            monitor: None,
        }
    }

    /// Render immediately. Workers own all collection; this loop only handles
    /// input and bounded latest-result delivery.
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.preferences = crate::settings::tui_preferences();
        self.preferences.no_color |= std::env::var_os("NO_COLOR").is_some();
        self.preferences.ascii |= std::env::var_os("SD300_ASCII").is_some()
            || std::env::var("TERM").is_ok_and(|s| s == "dumb");
        let _mouse = MouseSession::new(self.preferences.mouse_enabled)?;
        self.monitor = Some(Monitor::start(Profile::Full));
        let mut poll = interval(Duration::from_millis(50));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut events = crossterm::event::EventStream::new();
        let mut dirty = true;
        loop {
            if dirty {
                let size = terminal.size()?;
                self.too_small = size.width < 80 || size.height < 24;
                self.terminal_width = size.width;
                self.terminal_height = size.height;
                self.prepare_view();
                terminal.draw(|frame| ui::render(frame, self))?;
                dirty = false;
            }
            if self.should_quit {
                self.monitor.take();
                return Ok(());
            }
            tokio::select! {
                _ = poll.tick() => {
                    if self.exporter.poll() { dirty = true; }
                    if self.storage_probe.poll() {
                        self.live_while_paused.as_mut().unwrap_or(&mut self.snapshot).storage_probe = self.storage_probe.state.clone();
                        dirty = true;
                    }
                    if self.optional_setup.poll() {
                        self.live_while_paused.as_mut().unwrap_or(&mut self.snapshot).optional_setup = self.optional_setup.state.clone();
                        dirty = true;
                        if self.optional_setup.state.succeeded && self.optional_setup.state.tool == "smartctl" {
                            if let Some(monitor) = &self.monitor { monitor.retry(Lane::Health); }
                        }
                    }
                    if self.companion.poll() {
                        self.live_while_paused.as_mut().unwrap_or(&mut self.snapshot).companion = self.companion.state.clone();
                        dirty = true;
                    }
                    let target = self.live_while_paused.as_mut().unwrap_or(&mut self.snapshot);
                    let changed = self.monitor.as_ref().map(|m| m.drain(target)).unwrap_or_default();
                    if self.paused_at.is_none() {
                        if changed.contains(&Lane::Fast) { self.update_fast_history(); }
                        if changed.contains(&Lane::Slow) { self.update_slow_history(); }
                        if changed.contains(&Lane::Activity) { self.update_activity_history(); }
                        let now = crate::collectors::sampling::unix_ms();
                        let age_changed = now / 1000 != self.presentation_time_ms / 1000;
                        if !changed.is_empty() || age_changed {
                            self.presentation_time_ms = now;
                            self.findings = crate::findings::for_snapshot(&self.snapshot);
                            dirty = true;
                        }
                    }
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
        self.gpu_history
            .push_at(captured, self.snapshot.gpu.utilization().map(f64::from));
        // CPU and GPU temperatures must never be spliced into one series.
        self.temp_history
            .push_at(captured, self.snapshot.thermals.cpu_temp);
        self.gpu_temp_history
            .push_at(captured, self.snapshot.thermals.gpu_temp);
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
    pub fn prepare_view(&mut self) {
        self.view = crate::presentation::Presentation::prepare(self);
    }
    fn toggle_pause(&mut self) {
        if let Some(latest) = self.live_while_paused.take() {
            self.snapshot = latest;
            self.paused_at = None;
            self.presentation_time_ms = crate::collectors::sampling::unix_ms();
            self.update_fast_history();
            self.update_slow_history();
            self.update_activity_history();
            self.findings = crate::findings::for_snapshot(&self.snapshot);
        } else {
            self.live_while_paused = Some(self.snapshot.presentation_copy());
            self.paused_at = Some(self.presentation_time_ms);
        }
    }
    fn select_relative(&mut self, amount: isize) {
        self.view.selected = self
            .view
            .selected
            .saturating_add_signed(amount)
            .min(self.view.rows.len().saturating_sub(1));
        self.view.selected_id = self.view.rows.get(self.view.selected).map(|r| r.id.clone());
        self.inspector_scroll = 0;
        match self.current_section {
            Section::Processes => self.process_scroll = self.view.selected,
            Section::Network => self.connection_scroll = self.view.selected,
            Section::Drivers => self.driver_scroll = self.view.selected,
            Section::Disk => self.disk_scroll = self.view.selected,
            _ => {}
        }
    }
    fn select_section(&mut self, section: Section) {
        self.current_section = section;
        self.process_scroll = 0;
        self.connection_scroll = 0;
        self.driver_scroll = 0;
        self.disk_scroll = 0;
        self.filter.clear();
        self.editing_filter = false;
        self.inspector_scroll = 0;
        self.view.selected = 0;
        self.view.selected_id = None;
        self.show_inspector = false;
    }
    fn handle_event(&mut self, event: Event) {
        if let Event::Mouse(mouse) = event {
            if !self.preferences.mouse_enabled {
                return;
            }
            use crossterm::event::MouseEventKind;
            match mouse.kind {
                MouseEventKind::ScrollDown => self.select_relative(3),
                MouseEventKind::ScrollUp => self.select_relative(-3),
                MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                    if mouse.row == self.terminal_height.saturating_sub(1) {
                        let section = (mouse.column as usize * 9
                            / self.terminal_width.max(1) as usize)
                            .min(8);
                        self.select_section(Section::ALL[section]);
                    } else if mouse.row >= 11 && mouse.row < self.terminal_height.saturating_sub(2)
                    {
                        let height = self.terminal_height.saturating_sub(14).max(1) as usize;
                        let start = self.view.selected.saturating_sub(height.saturating_sub(1));
                        let index = start + mouse.row.saturating_sub(11) as usize;
                        self.select_relative(index as isize - self.view.selected as isize);
                    }
                }
                _ => {}
            }
            return;
        }
        let Event::Key(key) = event else { return };
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }
        if self.show_export {
            if key.kind != KeyEventKind::Press {
                return;
            }
            match key.code {
                KeyCode::Esc => self.show_export = false,
                KeyCode::Char('E' | 'C') => {
                    let kind = if key.code == KeyCode::Char('C') {
                        crate::export::Kind::Capabilities
                    } else {
                        crate::export::Kind::Snapshot
                    };
                    self.exporter.start(&self.snapshot, kind);
                }
                _ => {}
            }
            return;
        }
        if self.show_storage_probe {
            if key.kind == KeyEventKind::Repeat {
                return;
            }
            match key.code {
                KeyCode::Esc => {
                    self.storage_probe.cancel();
                    self.show_storage_probe = false;
                }
                KeyCode::Char('y' | 'Y') => {
                    self.storage_probe.confirm(true);
                }
                KeyCode::Char('x' | 'X') => self.storage_probe.cancel(),
                KeyCode::Down | KeyCode::Char('j') => {
                    self.inspector_scroll = self.inspector_scroll.saturating_add(1)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.inspector_scroll = self.inspector_scroll.saturating_sub(1)
                }
                _ => {}
            }
            return;
        }
        if self.show_companion {
            if key.kind == KeyEventKind::Repeat {
                return;
            }
            use crate::companion::Action;
            match key.code {
                KeyCode::Esc | KeyCode::Char('N') => {
                    self.show_companion = false;
                    self.setup_confirmation = false;
                    self.speed_confirmation = None;
                    self.mlab_consent = false;
                }
                KeyCode::Char('x' | 'X') => {
                    self.companion.cancel();
                    self.optional_setup.cancel();
                }
                KeyCode::Char('i' | 'h')
                    if !self.companion.state.running && !self.optional_setup.state.running =>
                {
                    self.setup_confirmation = true;
                    self.setup_smart = key.code == KeyCode::Char('h');
                    self.speed_confirmation = None;
                    self.mlab_consent = false;
                }
                KeyCode::Char('s')
                    if self.speed_confirmation.is_none()
                        && !self.setup_confirmation
                        && !self.optional_setup.state.running =>
                {
                    self.companion.start(Action::Standard, false);
                }
                KeyCode::Char('d')
                    if self.speed_confirmation.is_none()
                        && !self.setup_confirmation
                        && !self.optional_setup.state.running =>
                {
                    self.companion.start(Action::Deep, false);
                }
                KeyCode::Char('b')
                    if !self.setup_confirmation && !self.optional_setup.state.running =>
                {
                    self.speed_confirmation = Some(Action::SpeedQuick);
                    self.mlab_consent = false;
                }
                KeyCode::Char('B')
                    if !self.setup_confirmation && !self.optional_setup.state.running =>
                {
                    self.speed_confirmation = Some(Action::SpeedDeep);
                    self.mlab_consent = false;
                }
                KeyCode::Char('m') if self.speed_confirmation.is_some() => {
                    self.mlab_consent = !self.mlab_consent
                }
                KeyCode::Char('y' | 'Y') => {
                    if self.setup_confirmation {
                        self.setup_confirmation = false;
                        if self.setup_smart {
                            self.optional_setup.start_smart(true);
                        } else {
                            self.optional_setup.start_network(true);
                        }
                    } else if let Some(action) = self.speed_confirmation.take() {
                        self.companion.start(action, self.mlab_consent);
                        self.mlab_consent = false;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.inspector_scroll = self.inspector_scroll.saturating_add(1)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.inspector_scroll = self.inspector_scroll.saturating_sub(1)
                }
                _ => {}
            }
            return;
        }
        if self.editing_filter {
            self.view.selected_id = None;
            self.view.selected = 0;
            match key.code {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.editing_filter = false;
                }
                KeyCode::Enter => self.editing_filter = false,
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Char(c)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && self.filter.len() < 256 =>
                {
                    self.filter.push(c)
                }
                _ => {}
            }
            return;
        }
        if self.show_help {
            if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
                self.show_help = false;
            }
            return;
        }
        if self.show_findings {
            match key.code {
                KeyCode::Char('F') | KeyCode::Esc => {
                    self.show_findings = false;
                    self.inspector_scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.inspector_scroll = self.inspector_scroll.saturating_add(1)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.inspector_scroll = self.inspector_scroll.saturating_sub(1)
                }
                KeyCode::PageDown => {
                    self.inspector_scroll = self.inspector_scroll.saturating_add(10)
                }
                KeyCode::PageUp => self.inspector_scroll = self.inspector_scroll.saturating_sub(10),
                _ => {}
            }
            return;
        }
        if self.mode.is_none() {
            match key.code {
                KeyCode::Char('1') => self.mode = Some(DiagnosticMode::User),
                KeyCode::Char('2') => self.mode = Some(DiagnosticMode::Technician),
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                _ => {}
            }
            return;
        }
        // Repeat is useful for scrolling/filtering, but must not toggle pause or modes.
        if key.kind == KeyEventKind::Repeat
            && !matches!(
                key.code,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::PageDown
                    | KeyCode::PageUp
                    | KeyCode::Char('j' | 'k')
            )
        {
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc if self.show_inspector => {
                self.show_inspector = false;
                self.inspector_scroll = 0;
            }
            KeyCode::Esc if !self.filter.is_empty() => self.filter.clear(),
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('m') => self.mode = None,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('A') => {
                self.show_storage_probe = true;
                self.inspector_scroll = 0;
                if self.storage_probe.state.running {
                    return;
                }
                self.storage_probe.cancel();
                if self.paused_at.is_none()
                    && self
                        .snapshot
                        .samples
                        .get("health")
                        .is_some_and(|m| m.observation.is_available() && !m.is_stale())
                {
                    if let Some(crate::presentation::ViewRow {
                        target: crate::presentation::Target::Drive(index),
                        ..
                    }) = self.view.rows.get(self.view.selected)
                    {
                        if let Some(drive) = self.snapshot.disk_health.drives.get(*index) {
                            self.storage_probe.prepare(drive);
                        }
                    } else {
                        self.storage_probe.state.message = "Select a physical drive in Storage, then press A to prepare a privileged read".into();
                    }
                } else {
                    self.storage_probe.state.message = "Resume the live view and wait for a current storage inventory before requesting a privileged read".into();
                }
            }
            KeyCode::Char('E') => {
                self.show_export = true;
            }
            KeyCode::Char('N') => {
                self.show_companion = true;
                self.inspector_scroll = 0;
            }
            KeyCode::Char('F') => {
                self.show_findings = true;
                self.inspector_scroll = 0;
            }
            KeyCode::Char(' ') => self.toggle_pause(),
            KeyCode::Char('/') => self.editing_filter = true,
            KeyCode::Enter => {
                self.show_inspector = !self.show_inspector;
                self.inspector_scroll = 0;
            }
            KeyCode::Char(c @ '1'..='9') => {
                if let Some(s) = Section::from_number(c as u8 - b'0') {
                    self.select_section(s);
                }
            }
            KeyCode::Tab => {
                self.select_section(Section::ALL[(self.current_section.number() as usize) % 9])
            }
            KeyCode::BackTab => {
                self.select_section(Section::ALL[(self.current_section.number() as usize + 7) % 9])
            }
            KeyCode::Down | KeyCode::Char('j') if self.show_inspector => {
                self.inspector_scroll = self.inspector_scroll.saturating_add(1)
            }
            KeyCode::Up | KeyCode::Char('k') if self.show_inspector => {
                self.inspector_scroll = self.inspector_scroll.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') => self.select_relative(1),
            KeyCode::Up | KeyCode::Char('k') => self.select_relative(-1),
            KeyCode::PageDown if self.show_inspector => {
                self.inspector_scroll = self.inspector_scroll.saturating_add(10)
            }
            KeyCode::PageUp if self.show_inspector => {
                self.inspector_scroll = self.inspector_scroll.saturating_sub(10)
            }
            KeyCode::PageDown => {
                self.select_relative(self.terminal_height.saturating_sub(14).max(1) as isize)
            }
            KeyCode::PageUp => {
                self.select_relative(-(self.terminal_height.saturating_sub(14).max(1) as isize))
            }
            KeyCode::Home => self.select_relative(-(self.view.selected as isize)),
            KeyCode::End => self.select_relative(self.view.rows.len() as isize),
            KeyCode::Char('c') if self.current_section == Section::Processes => {
                self.process_sort = ProcessSortKey::Cpu;
                self.sort_reversed = false;
            }
            KeyCode::Char('M') if self.current_section == Section::Processes => {
                self.process_sort = ProcessSortKey::Memory;
                self.sort_reversed = false;
            }
            KeyCode::Char('n') if self.current_section == Section::Processes => {
                self.process_sort = ProcessSortKey::Name;
                self.sort_reversed = false;
            }
            KeyCode::Char('p') if self.current_section == Section::Processes => {
                self.process_sort = ProcessSortKey::Pid;
                self.sort_reversed = false;
            }
            KeyCode::Char('s') if self.current_section == Section::Processes => {
                self.sort_reversed = !self.sort_reversed
            }
            KeyCode::Char('f') => self.temp_unit = self.temp_unit.toggle(),
            KeyCode::Char('r') => {
                if let Some(m) = &self.monitor {
                    for lane in Lane::ALL {
                        m.retry(lane);
                    }
                }
            }
            _ => {}
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

struct MouseSession(bool);
impl MouseSession {
    fn new(enabled: bool) -> std::io::Result<Self> {
        if enabled {
            crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
        }
        Ok(Self(enabled))
    }
}
impl Drop for MouseSession {
    fn drop(&mut self) {
        if self.0 {
            let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
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
        app.prepare_view();
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
    fn opening_export_is_explicit_and_does_not_start_disk_io() {
        let mut app = App::new(Some(DiagnosticMode::User));
        press(&mut app, KeyCode::Char('E'));
        assert!(app.show_export);
        assert!(!app.exporter.running());
        assert!(app.exporter.result.is_none());
        press(&mut app, KeyCode::Esc);
        assert!(!app.show_export);
        assert!(!app.should_quit);
    }

    #[test]
    fn storage_action_requires_an_inventory_selection_and_distinct_consent() {
        let mut app = App::new(Some(DiagnosticMode::User));
        press(&mut app, KeyCode::Char('A'));
        assert!(app.show_storage_probe);
        assert!(!app.storage_probe.state.running);
        assert!(!app.storage_probe.state.awaiting_consent);
        press(&mut app, KeyCode::Char('Y'));
        assert!(!app.storage_probe.state.running);
        press(&mut app, KeyCode::Esc);
        assert!(!app.show_storage_probe);
        assert!(!app.should_quit);
    }

    #[test]
    fn opening_companion_or_speed_confirmation_never_launches_a_probe() {
        let mut app = App::new(Some(DiagnosticMode::User));
        press(&mut app, KeyCode::Char('N'));
        assert!(app.show_companion);
        assert!(!app.companion.state.running);
        press(&mut app, KeyCode::Char('b'));
        assert_eq!(
            app.speed_confirmation,
            Some(crate::companion::Action::SpeedQuick)
        );
        assert!(!app.mlab_consent);
        assert!(!app.companion.state.running);
        press(&mut app, KeyCode::Char('m'));
        assert!(app.mlab_consent);
        press(&mut app, KeyCode::Esc);
        assert!(!app.show_companion);
        assert!(!app.mlab_consent);
        assert!(!app.companion.state.running);
    }

    #[test]
    fn optional_setup_opens_without_consent_and_escape_declines() {
        let mut app = App::new(Some(DiagnosticMode::User));
        press(&mut app, KeyCode::Char('N'));
        press(&mut app, KeyCode::Char('b'));
        press(&mut app, KeyCode::Char('m'));
        press(&mut app, KeyCode::Char('i'));
        assert!(app.setup_confirmation);
        assert!(app.speed_confirmation.is_none());
        assert!(!app.mlab_consent);
        assert!(!app.optional_setup.state.running);
        press(&mut app, KeyCode::Esc);
        assert!(!app.setup_confirmation);
        assert!(!app.optional_setup.state.running);
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
    fn section_unit_sort_keys_remain_and_v4_selection_is_bounded() {
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
        assert_eq!(
            app.connection_scroll, 2,
            "connectivity evidence is an additive row"
        );
        press(&mut app, KeyCode::Up);
        assert_eq!(app.connection_scroll, 1);

        app.current_section = Section::Drivers;
        press(&mut app, KeyCode::Down);
        assert_eq!(
            app.driver_scroll, 0,
            "empty inventories cannot select nonexistent rows"
        );
        press(&mut app, KeyCode::Up);
        assert_eq!(app.driver_scroll, 0);

        app.current_section = Section::Disk;
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(
            app.disk_scroll, 0,
            "empty inventories cannot select nonexistent rows"
        );
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
    #[test]
    fn full_inventory_filter_sort_and_pid_reuse_keep_selection_truthful() {
        let mut app = App::new(Some(DiagnosticMode::Technician));
        app.select_section(Section::Processes);
        app.snapshot.processes.list = (1..=250)
            .map(|pid| {
                let mut p = process(pid);
                p.start_time_unix_ms = Some(pid as u64);
                p.memory_bytes = pid as u64 * 1024;
                p.memory_observation = crate::observation::Observation::available("fixture");
                p
            })
            .collect();
        press(&mut app, KeyCode::Char('M'));
        app.prepare_view();
        assert!(app.view.rows[0].name.ends_with("250"));
        app.select_relative(11);
        let selected = app.view.selected_id.clone();
        app.snapshot.processes.list.reverse();
        app.prepare_view();
        assert_eq!(app.view.selected_id, selected);
        let target = app.view.rows[app.view.selected].target;
        if let crate::presentation::Target::Process(i) = target {
            app.snapshot.processes.list[i].start_time_unix_ms = Some(99999);
        }
        app.prepare_view();
        assert!(app.view.selection_lost);
        assert!(app.view.inspector[0].contains("ended"));
        press(&mut app, KeyCode::Char('/'));
        for c in "process-200".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Enter);
        app.prepare_view();
        assert_eq!(app.view.rows.len(), 1);
        assert_eq!(app.view.rows[0].name, "process-200");
    }
    #[test]
    fn pause_freezes_values_while_latest_collection_can_advance() {
        let mut app = App::new(Some(DiagnosticMode::User));
        app.snapshot.cpu.total_usage = 20.0;
        app.toggle_pause();
        let frozen = app.paused_at;
        app.live_while_paused.as_mut().unwrap().cpu.total_usage = 80.0;
        app.prepare_view();
        assert_eq!(app.snapshot.cpu.total_usage, 20.0);
        assert_eq!(app.paused_at, frozen);
        app.toggle_pause();
        assert_eq!(app.snapshot.cpu.total_usage, 80.0);
        assert!(app.paused_at.is_none());
    }
    #[test]
    fn repeat_scrolls_but_does_not_toggle_pause() {
        let mut app = App::new(Some(DiagnosticMode::User));
        app.handle_event(Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )));
        assert!(app.paused_at.is_none());
    }
}
