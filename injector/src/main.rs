mod platform;

use eframe::{CreationContext, Frame};
use egui::Context;
use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use std::fs::File;

fn main() {
    // Initialize the logger with a default configuration
    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create("app.log").unwrap(),
    )
    .unwrap();

    if !is_elevated() {
        #[cfg(target_family = "unix")]
        eprintln!("❌ Please run this program with sudo: `sudo ./injector`");

        #[cfg(target_family = "windows")]
        eprintln!("❌ Please run this program as Administrator (Right click → Run as administrator)");

        return; // non lancio la GUI
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "DarkClient Injector",
        native_options,
        Box::new(|creation_context| Ok(Box::new(InjectorGUI::new(creation_context)))),
    )
    .expect("Failed to run the GUI");
}

pub struct InjectorGUI {
    status: String,
    pid: Option<u32>,
}

impl InjectorGUI {
    pub fn new(_creation_context: &CreationContext<'_>) -> Self {
        Self {
            status: "Hello, welcome to DarkClient Injector:".to_owned(),
            pid: None,
        }
    }
}

impl eframe::App for InjectorGUI {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DarkClient Injector");

            ui.label("Status: ".to_owned() + &self.status);

            if ui.button("find").clicked() {
                self.pid = platform::find_pid();
                if self.pid.is_none() {
                    self.status = "Failed to find PID".to_owned();
                } else {
                    self.status = format!("Found PID: {}", self.pid.unwrap());
                }
            }

            if ui.button("Inject").clicked() {
                if self.pid.is_none() {
                    self.status = "Please find the PID first".to_owned();
                    return;
                }
                match platform::inject(self.pid.unwrap()) {
                    Ok(_) => self.status = "Injected successfully!".to_owned(),
                    Err(e) => {
                        log::error!("Error during injection: {:?}", e);
                        self.status = format!("Failed to inject: {}", e)
                    }
                }
            }
        });
    }
}

#[cfg(target_family = "unix")]
fn is_elevated() -> bool {
    extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() == 0 }
}

#[cfg(target_family = "windows")]
fn is_elevated() -> bool {
    extern "system" {
        fn IsUserAnAdmin() -> i32;
    }
    unsafe { IsUserAnAdmin() != 0 }
}