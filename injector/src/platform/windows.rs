use crate::platform::{AGENT_NAME, LIBRARY_NAME, SOCKET_ADDRESS};
use std::{io, path, thread};
use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use log::{error, info};
use proc_maps::get_process_maps;

pub fn inject(pid: u32) -> Result<(), io::Error> {
    let loader_path = PathBuf::from(format!("{}.dll", AGENT_NAME));
    let lib_path = PathBuf::from(format!("{}.dll", LIBRARY_NAME));

    // Check if agent_loader is already loaded
    if !find_library(pid, "agent_loader") {
        info!("Loading Agent Loader");

        // Load agent_loader via JVMTI
        match Command::new("jcmd")
            .arg(pid.to_string())
            .arg("JVMTI.agent_load")
            .arg(format!("{:?}", path::absolute(&loader_path)?))
            .output()
        {
            Ok(output) if output.status.success() => {
                info!("Agent Loader loaded via jcmd: {:?}", loader_path);
            }
            Ok(output) => {
                error!(
                    "jcmd failed (stderr): {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                error!("Unable to execute jcmd: {:?}", e);
            }
        }

        // Wait a moment for complete initialization
        thread::sleep(Duration::from_millis(500));
    } else {
        info!("Agent Loader already loaded");
    }

    // Send a reload command to agent_loader
    match TcpStream::connect_timeout(&SOCKET_ADDRESS, Duration::from_secs(5)) {
        Ok(mut stream) => {
            let lib_abs_path = match path::absolute(&lib_path) {
                Ok(p) => p,
                Err(e) => {
                    error!("Unable to get absolute path: {:?}", e);
                    return Err(e);
                }
            };

            info!("Connected to {}. Sending reload command", SOCKET_ADDRESS);

            let lib_abs_path = lib_abs_path.to_string_lossy();
            let lib_abs_path = lib_abs_path.trim_matches(|c| c == '"' || c == '\'');
            // Send the command with the absolute path of the library
            let command = format!("reload {}", lib_abs_path);
            info!("Command: {}", command);

            if let Err(e) = stream.write(command.as_bytes()) {
                error!("Unable to send reload command: {:?}", e);
            }
        }
        Err(e) => {
            error!("Unable to connect to server: {:?}", e);
        }
    }

    Ok(())
}

pub fn find_pid() -> Option<u32> {
    let output = Command::new("tasklist")
        .output()
        .expect("Failed to execute `tasklist` command");

    let output_str = String::from_utf8_lossy(&output.stdout);

    for line in output_str.lines() {
        if line.contains("minecraft") && line.contains("java") {
            if let Some(pid) = line.split_whitespace().nth(1) {
                println!("{}", pid);
            }
        }
    }
    None
}

fn find_library(pid: u32, lib_name: &str) -> bool {
    let maps = get_process_maps(pid).ok();
    if maps.is_none() {
        return false;
    }
    let maps = maps.unwrap();

    for map in maps {
        if let Some(path) = map.filename() {
            if path.ends_with(format!("{}.dll", lib_name)) {
                // Library loaded
                return true;
            }
        }
    }
    false
}