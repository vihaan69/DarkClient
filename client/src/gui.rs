use crate::LogExpect;
use eframe::Frame;
use egui::Context;
use winit::platform::x11::EventLoopBuilderExtX11;

pub fn start_gui() {
    let mut native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        run_and_return: true,
        ..Default::default()
    };

    native_options.event_loop_builder = Some(Box::new(|builder| {
        #[cfg(target_os = "linux")]
        {
            builder.with_x11().with_any_thread(true);
        }
    }));

    eframe::run_native(
        "DarkClient Injector",
        native_options,
        Box::new(|_| Ok(Box::new(GUI::default()))),
    )
    .log_expect("Failed to run the GUI");
}

pub struct GUI;

impl Default for GUI {
    fn default() -> Self {
        Self
    }
}

impl eframe::App for GUI {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DarkClient");

            ui.label("Status: Injected");
        });
    }
}
