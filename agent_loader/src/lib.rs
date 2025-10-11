extern crate ctor;
extern crate log;
extern crate simplelog;

use ctor::*;
use libloading::{Library, Symbol};
use log::{error, info, LevelFilter};
use simplelog::{Config, WriteLogger};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use std::{path, thread};
use jni::JavaVM;
use jni::sys::{jsize, JNI_GetCreatedJavaVMs, JNI_OK};

// Global variable to keep track of the loaded library
static CLIENT_LIBRARY: OnceLock<Mutex<Option<Library>>> = OnceLock::new();
static RUNNING: AtomicBool = AtomicBool::new(true);
static JVM_MONITOR: OnceLock<thread::JoinHandle<()>> = OnceLock::new();

// Function called when the agent is loaded
#[no_mangle]
#[ctor]
fn agent_onload() {
    // Initialize the logger
    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create("agent_loader.log").unwrap(),
    )
    .unwrap();

    info!("Agent Loader initialized");

    // Initialize the global variable for the library
    CLIENT_LIBRARY.get_or_init(|| Mutex::new(None));

    // Setup signal handlers for clean shutdown
    setup_signal_handlers();

    // Start monitoring the JVM
    start_jvm_monitor();

    // Start the socket server for commands
    start_command_server();
}

// Function called when the agent is unloaded
#[no_mangle]
#[dtor]
fn agent_onunload() {
    if !RUNNING.load(Ordering::SeqCst) {
        return;
    }
    info!("Agent Loader unloading");
    RUNNING.store(false, Ordering::SeqCst);

    // Unload the client library if necessary
    if let Some(mut guard) = CLIENT_LIBRARY.get().and_then(|m| m.lock().ok()) {
        *guard = None;
    }
}

// Setup signal handlers to detect process termination
fn setup_signal_handlers() {
    use std::sync::atomic::AtomicBool;

    static SIGNAL_HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

    if SIGNAL_HANDLER_INSTALLED.swap(true, Ordering::SeqCst) {
        return; // Already installed
    }

    #[cfg(unix)]
    {
        extern "C" fn handle_signal(_: libc::c_int) {
            info!("Received termination signal - cleaning up");
            agent_onunload();
            std::process::exit(0);
        }

        unsafe {
            libc::signal(libc::SIGTERM, handle_signal as libc::sighandler_t);
            libc::signal(libc::SIGINT, handle_signal as libc::sighandler_t);
        }

        info!("Signal handlers installed");
    }
}

// Monitor the JVM status with multiple detection methods
fn start_jvm_monitor() {
    let handle = thread::spawn(|| {
        info!("JVM monitor thread started");

        // Wait for JVM to be available
        let jvm = loop {
            if !RUNNING.load(Ordering::SeqCst) {
                return;
            }

            match get_jvm() {
                Some(vm) => break vm,
                None => {
                    thread::sleep(Duration::from_millis(500));
                }
            }
        };

        info!("JVM detected, monitoring started");

        // Monitor JVM health with multiple checks
        let mut consecutive_failures = 0;
        let max_failures = 3;

        while RUNNING.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(500));

            // Method 1: Try to attach to the JVM
            let attach_ok = jvm.attach_current_thread_as_daemon().is_ok();

            // Method 2: Check if we can access Java classes
            let classes_ok = if attach_ok {
                if let Ok(mut env) = jvm.attach_current_thread_as_daemon() {
                    env.find_class("java/lang/System").is_ok()
                } else {
                    false
                }
            } else {
                false
            };

            // Method 3: Check if the JVM pointer is still valid
            let jvm_valid = {
                let jvm_ptr = jvm.get_java_vm_pointer();
                !jvm_ptr.is_null()
            };

            if !attach_ok || !classes_ok || !jvm_valid {
                consecutive_failures += 1;
                info!(
                    "JVM health check failed ({}/{}): attach={}, classes={}, valid={}",
                    consecutive_failures, max_failures, attach_ok, classes_ok, jvm_valid
                );

                if consecutive_failures >= max_failures {
                    info!("JVM appears to be shutting down or dead");
                    on_vm_death();
                    break;
                }
            } else {
                consecutive_failures = 0;
            }
        }

        info!("JVM monitor thread stopped");
    });

    JVM_MONITOR.set(handle).ok();
}

fn get_jvm() -> Option<JavaVM> {
    unsafe {
        let mut java_vm: *mut jni::sys::JavaVM = std::ptr::null_mut();
        let mut count: jsize = 0;

        if JNI_GetCreatedJavaVMs(&mut java_vm, 1, &mut count) != JNI_OK || count == 0 {
            return None;
        }

        JavaVM::from_raw(java_vm).ok()
    }
}

fn on_vm_death() {
    info!("VM death detected - initiating cleanup");
    agent_onunload();
}

// Function to load the client library
fn load_client_library(lib_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client_path = PathBuf::from(lib_path);

    // Verify the path
    info!("Verifying library path: {:?}", client_path);

    // Unload first
    unload_client_library()?;

    if !client_path.exists() {
        error!("Client library does not exist at path: {:?}", client_path);
        if let Ok(abs_path) = path::absolute(&client_path) {
            error!("Absolute path: {:?}", abs_path);
        }
        return Err(format!("Client library does not exist at path: {:?}", client_path).into());
    }

    info!("Loading client library: {:?}", client_path);

    // Get the lock on the global variable
    let mut lib_guard = CLIENT_LIBRARY.get().unwrap().lock().unwrap();

    // Unload the previous library if present
    if lib_guard.is_some() {
        *lib_guard = None;
    }

    // Load the new library
    let lib = unsafe { Library::new(&client_path)? };

    // Find and call the initialization function
    unsafe {
        if let Ok(init_fn) = lib.get::<Symbol<extern "C" fn()>>(b"initialize_client") {
            info!("Calling initialization function");
            init_fn();
        } else {
            info!("Initialization function not found, assuming self-initialization");
        }
    }

    // Store the library
    *lib_guard = Some(lib);

    info!("Client library loaded successfully");
    Ok(())
}

// Function to unload the client library
fn unload_client_library() -> Result<(), Box<dyn std::error::Error>> {
    info!("Unloading client library");

    let mut lib_guard = CLIENT_LIBRARY.get().unwrap().lock().unwrap();

    if let Some(lib) = lib_guard.as_ref() {
        // Call the cleanup function if present
        unsafe {
            if let Ok(cleanup_fn) = lib.get::<Symbol<extern "C" fn()>>(b"cleanup_client") {
                info!("Calling cleanup function");
                cleanup_fn();
            }
        }

        drop(lib_guard.take());

        // Unload the library
        *lib_guard = None;

        info!("Client library unloaded");
    } else {
        info!("No client library loaded");
    }

    Ok(())
}

// Function to reload the client library
fn reload_client_library(lib_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Reloading client library");

    // Copy the file if necessary to avoid lock issues
    let client_path = PathBuf::from(lib_path);
    let filename = client_path
        .file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("Invalid file name (Unicode)")?;

    // Generate a timestamp for the temporary copy
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Temporary file name
    let temp_filename = format!("temp_{}_{}", timestamp, filename);
    let temp_path = PathBuf::from(&temp_filename);

    // Copy the file
    std::fs::copy(&client_path, &temp_path)?;
    info!("Library copied to: {:?}", temp_path);

    let temp_client =
        path::absolute(&temp_path).map_err(|e| format!("Unable to get absolute path: {:?}", e))?;
    let temp_client = format!("{:?}", temp_client.to_string_lossy());
    let temp_client = temp_client.as_str();
    let temp_client = temp_client.trim_matches(|c| c == '"' || c == '\'');

    // Cleanup of temporary files (optional, can be executed in a separate thread)
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(5)); // Wait a bit before deleting
        if let Err(e) = std::fs::remove_file(&temp_path) {
            error!("Unable to delete temporary file: {:?}", e);
        }
    });

    // Load the new copy
    load_client_library(temp_client)?;

    // Wait a bit to ensure all resources are released
    thread::sleep(Duration::from_millis(100));

    info!("Client library reloaded successfully");
    Ok(())
}

// Start a socket server to listen for commands
fn start_command_server() {
    thread::spawn(move || {
        let addr = "127.0.0.1:7878";
        let listener = match TcpListener::bind(addr) {
            Ok(listener) => {
                info!("Listening on {}", addr);
                listener
            }
            Err(e) => {
                error!("Unable to bind to {}: {}", addr, e);
                return;
            }
        };

        // Set the socket to non-blocking mode
        listener.set_nonblocking(true).unwrap();

        while RUNNING.load(Ordering::SeqCst) {
            // Check for incoming connections
            match listener.accept() {
                Ok((stream, _)) => {
                    let mut reader = BufReader::new(stream);
                    let mut line = String::new();

                    if reader.read_line(&mut line).is_ok() {
                        let line = line.trim();
                        let parts: Vec<&str> = line.splitn(2, ' ').collect();

                        match parts.first() {
                            Some(&"reload") => {
                                if let Some(path) = parts.get(1) {
                                    info!("Reload command received with path: {}", path);

                                    if let Err(e) = reload_client_library(path) {
                                        error!("Error during reload: {}", e);
                                    }
                                } else {
                                    error!("Reload command received without path!");
                                }
                            }
                            Some(other) => {
                                error!("Unknown command: {}", other);
                            }
                            None => {
                                error!("Empty command received");
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection available, wait a bit
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    error!("Error while accepting connection: {}", e);
                    // Short pause to avoid infinite loops in case of errors
                    thread::sleep(Duration::from_millis(1000));
                }
            }
        }
    });
}
