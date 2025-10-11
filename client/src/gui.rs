use crate::client::DarkClient;
use crate::module::{ModuleCategory, ModuleSetting};
use crate::{cleanup_client, RUNNING};
use eframe::Frame;
use egui::{Context, ScrollArea, Ui};
use std::sync::atomic::Ordering::Relaxed;
#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

pub fn call_panic() {
    let client = DarkClient::instance();
    client.modules.read().unwrap().values().for_each(|module| {
        let mut module = module.lock().unwrap();
        if module.get_module_data().enabled {
            module.get_module_data_mut().set_enabled(false);
            match module.on_stop() {
                Ok(_) => {}
                Err(e) => {
                    log::error!(
                        "Failed to stop module {} on panic: {}",
                        module.get_module_data().name,
                        e
                    );
                }
            }
        }
    });
    cleanup_client();
}

pub fn start_gui() -> anyhow::Result<()> {
    let mut native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([700.0, 500.0]),
        run_and_return: true,
        ..Default::default()
    };

    #[cfg(target_os = "linux")]
    {
        native_options.event_loop_builder = Some(Box::new(|builder| {
            builder.with_x11().with_any_thread(true);
        }));
    }

    match eframe::run_native(
        "DarkClient Injector",
        native_options,
        Box::new(|_| Ok(Box::new(GUI::default()))),
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("Failed to run the GUI, {}", e)),
    }
}

pub struct GUI {
    selected_category: ModuleCategory,
}

impl Default for GUI {
    fn default() -> Self {
        Self {
            selected_category: ModuleCategory::COMBAT,
        }
    }
}

impl eframe::App for GUI {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        ctx.request_repaint();

        if !RUNNING.load(Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DarkClient");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.colored_label(egui::Color32::GREEN, "Injected");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Panic").clicked() {
                        std::thread::spawn(|| call_panic());
                    }
                });
            });

            ui.add_space(10.0);

            // Category selection
            ui.horizontal(|ui| {
                ui.label("Category:");
                if ui
                    .selectable_label(self.selected_category == ModuleCategory::COMBAT, "⚔ Combat")
                    .clicked()
                {
                    self.selected_category = ModuleCategory::COMBAT;
                }
                if ui
                    .selectable_label(
                        self.selected_category == ModuleCategory::MOVEMENT,
                        "🏃 Movement",
                    )
                    .clicked()
                {
                    self.selected_category = ModuleCategory::MOVEMENT;
                }
                if ui
                    .selectable_label(self.selected_category == ModuleCategory::RENDER, "👁 Render")
                    .clicked()
                {
                    self.selected_category = ModuleCategory::RENDER;
                }
                if ui
                    .selectable_label(
                        self.selected_category == ModuleCategory::PLAYER,
                        "🧍 Player",
                    )
                    .clicked()
                {
                    self.selected_category = ModuleCategory::PLAYER;
                }
                if ui
                    .selectable_label(self.selected_category == ModuleCategory::WORLD, "🌍 World")
                    .clicked()
                {
                    self.selected_category = ModuleCategory::WORLD;
                }
                if ui
                    .selectable_label(self.selected_category == ModuleCategory::MISC, "🔧 Misc")
                    .clicked()
                {
                    self.selected_category = ModuleCategory::MISC;
                }
            });

            ui.separator();

            // Modules list
            ScrollArea::vertical().show(ui, |ui| {
                self.render_modules(ui);
            });
        });
    }
}

impl GUI {
    fn render_modules(&mut self, ui: &mut Ui) {
        let client = DarkClient::instance();
        let modules = client.modules.read().unwrap();

        let mut modules_in_category: Vec<_> = modules
            .iter()
            .filter(|(_, module)| {
                module.lock().unwrap().get_module_data().category == self.selected_category
            })
            .collect();

        modules_in_category.sort_by(|a, b| {
            a.1.lock()
                .unwrap()
                .get_module_data()
                .name
                .cmp(&b.1.lock().unwrap().get_module_data().name)
        });

        if modules_in_category.is_empty() {
            ui.label("No modules in this category");
            return;
        }

        for (_, module) in modules_in_category {
            let mut module = module.lock().unwrap();

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let mut enabled = module.get_module_data().enabled;
                    if ui.checkbox(&mut enabled, "").changed() {
                        if enabled {
                            match module.on_start() {
                                Ok(_) => {
                                    module.get_module_data_mut().set_enabled(true);
                                }
                                Err(e) => {
                                    log::error!("Failed to start module: {}", e);
                                }
                            }
                        } else {
                            match module.on_stop() {
                                Ok(_) => {
                                    module.get_module_data_mut().set_enabled(false);
                                }
                                Err(e) => {
                                    log::error!("Failed to stop module: {}", e);
                                }
                            }
                        }
                    }

                    let module_data = module.get_module_data();
                    ui.vertical(|ui| {
                        ui.strong(&module_data.name);
                        ui.label(&module_data.description);
                        ui.label(format!("Keybind: {:?}", module_data.key_bind));
                    });
                });

                let module_data = module.get_module_data();
                // Render module settings
                if module_data.enabled {
                    ui.separator();
                    self.render_module_settings(ui, &mut *module);
                }
            });

            ui.add_space(5.0);
        }
    }

    fn render_module_settings(&mut self, ui: &mut Ui, module: &mut dyn crate::module::Module) {
        let module_data = module.get_module_data_mut();

        if module_data.settings.is_empty() {
            return;
        }

        ui.label("⚙ Settings:");
        ui.indent("settings", |ui| {
            let settings_len = module_data.settings.len();
            for i in 0..settings_len {
                let setting = &mut module_data.settings[i];

                match setting {
                    ModuleSetting::Slider {
                        name,
                        value,
                        min,
                        max,
                    } => {
                        ui.horizontal(|ui| {
                            ui.label(name.as_str());
                            let mut temp_value = *value;
                            if ui
                                .add(
                                    egui::Slider::new(&mut temp_value, *min..=*max)
                                        .fixed_decimals(1),
                                )
                                .changed()
                            {
                                *value = temp_value;
                            }
                        });
                    }
                    ModuleSetting::Toggle { name, value } => {
                        ui.horizontal(|ui| {
                            let mut temp_value = *value;
                            if ui.checkbox(&mut temp_value, name.as_str()).changed() {
                                *value = temp_value;
                            }
                        });
                    }
                    ModuleSetting::Choice {
                        name,
                        value,
                        options,
                    } => {
                        ui.horizontal(|ui| {
                            ui.label(name.as_str());
                            egui::ComboBox::from_id_salt(format!("choice_{}", name))
                                .selected_text(&options[*value])
                                .show_ui(ui, |ui| {
                                    for (idx, option) in options.iter().enumerate() {
                                        ui.selectable_value(value, idx, option);
                                    }
                                });
                        });
                    }
                    ModuleSetting::Color { name, value } => {
                        ui.horizontal(|ui| {
                            ui.label(name.as_str());
                            let mut color = egui::Color32::from_rgba_unmultiplied(
                                (value[0] * 255.0) as u8,
                                (value[1] * 255.0) as u8,
                                (value[2] * 255.0) as u8,
                                (value[3] * 255.0) as u8,
                            );
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                let rgba = color.to_srgba_unmultiplied();
                                value[0] = rgba[0] as f32 / 255.0;
                                value[1] = rgba[1] as f32 / 255.0;
                                value[2] = rgba[2] as f32 / 255.0;
                                value[3] = rgba[3] as f32 / 255.0;
                            }
                        });
                    }
                }
            }
        });
    }
}
