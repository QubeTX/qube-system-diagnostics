//! Standalone native producer for worker pipe qualification; no real probes.
use std::io::{Read, Write};
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).is_some_and(|arg| arg == "hold") {
        std::thread::sleep(std::time::Duration::from_secs(30));
        return;
    }
    let mode = args.get(2).expect("fixed fixture topic");
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    while input.read_exact(&mut [0; 2]).is_ok() {
        if mode == "invalid" {
            output.write_all(b"not-a-frame").unwrap();
            output.flush().unwrap();
        } else if mode == "noisy" {
            std::io::stderr().write_all(&vec![b'x'; 9 * 1024 * 1024]).unwrap();
        } else {
            let length: u32 = match mode.as_str() {
                "large" => 1024 * 1024,
                "oversized" => 8 * 1024 * 1024 + 1,
                _ => 1024,
            };
            for byte in b"SD4\0".iter().chain(length.to_le_bytes().iter()) {
                output.write_all(&[*byte]).unwrap();
                output.flush().unwrap();
            }
            if mode == "large" {
                output.write_all(&vec![b'a'; length as usize]).unwrap();
                output.flush().unwrap();
                continue;
            }
            output.write_all(b"partial").unwrap();
            output.flush().unwrap();
            if mode == "inherited" {
                std::process::Command::new(std::env::current_exe().unwrap())
                    .arg("hold").stdin(std::process::Stdio::null()).spawn().unwrap();
                return;
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
}
