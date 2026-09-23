// Native fixture: never opens a device. The parent verifies the exact read-only
// argument contract, structured fault import, disconnect cancellation and exit.
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 || args[..3] != ["--json", "--all", "--nocheck=standby,3"] {
        std::process::exit(2);
    }
    if args[3].contains("99999998") {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
    println!(r#"{{"smartctl":{{"exit_status":8}},"serial_number":"FIXTURE-ALPHA","temperature":{{"current":41}},"smart_status":{{"passed":false}}}}"#);
    std::process::exit(8);
}
