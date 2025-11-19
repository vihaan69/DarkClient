use crate::mapping::entity::player::LocalPlayer;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleSetting};

#[derive(Debug)]
pub struct FlyModule {
    pub module: ModuleData,
}

impl FlyModule {
    pub fn new(player: LocalPlayer) -> Self {
        Self {
            module: ModuleData {
                name: "Fly".to_string(),
                description: "Enables flying".to_string(),
                category: ModuleCategory::MOVEMENT,
                key_bind: KeyboardKey::KeyF,
                enabled: true,
                player,
                settings: vec![ModuleSetting::Slider {
                    name: "Speed".to_string(),
                    value: 1.0,
                    min: 0.1,
                    max: 3.0,
                }],
            },
        }
    }

    pub fn get_speed(&self) -> f32 {
        self.module
            .get_setting("Speed")
            .and_then(|s| s.get_slider_value())
            .unwrap_or(1.0)
    }
}

impl Module for FlyModule {
    fn on_start(&self) -> anyhow::Result<()> {
        // Enables flying
        self.module.player.abilities.fly(true)
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        // Disables flying
        self.module.player.abilities.fly(false)
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        // No operation
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
