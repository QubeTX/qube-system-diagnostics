use clap::Parser;

/// SD-300 System Diagnostic — Real-time interactive system diagnostics TUI
#[derive(Parser, Debug)]
#[command(name = "sd300")]
#[command(author, version, about = "SD-300 System Diagnostic — QubeTX Developer Tools")]
#[command(long_about = "SD-300 is a live, interactive terminal user interface for real-time \n\
    system diagnostics and monitoring. Part of the QubeTX 300 Series \n\
    alongside TR-300 (Machine Report) and ND-300 (Network Diagnostic).\n\n\
    Run without flags to choose a diagnostic mode interactively, \n\
    or use --user / --tech to launch directly into a mode.")]
#[command(after_long_help = "\
KEYBINDINGS:
  1-9          Switch to section
  q / Esc      Quit
  Ctrl+C       Quit to shell
  m            Return to mode selection
  ?            Help overlay
  f            Toggle temperature unit (C/F)
  j / k        Scroll (Processes, Connections, Drivers, Disk)
  c / M / p / n  Sort processes by CPU / Memory / PID / Name
  r            Refresh drivers (Drivers section)

SECTIONS:
  1 Overview    System health dashboard / identity and gauges
  2 CPU         Load, sparkline, per-core utilization
  3 Memory      Usage, swap, top consumers
  4 Disk        Drive health, partitions, SMART data
  5 GPU         Utilization, VRAM, temperature
  6 Network     Connectivity, interfaces, active connections
  7 Processes   Running apps / sortable process table
  8 Thermals    Temperature, fans, battery, power
  9 Drivers     Device health, driver versions, services")]
pub struct Cli {
    /// Launch directly into User Mode (plain language diagnostics)
    #[arg(long)]
    pub user: bool,

    /// Launch directly into Technician Mode (raw data and advanced metrics)
    #[arg(long)]
    pub tech: bool,
}
