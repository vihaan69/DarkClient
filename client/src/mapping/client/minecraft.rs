use crate::mapping::client::window::Window;
use crate::mapping::client::world::World;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::{Mapping, MinecraftClassType};
use jni::objects::GlobalRef;
use log::error;
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

#[derive(Debug)]
pub struct Minecraft {
    pub jni_ref: GlobalRef,
    mapping: Mapping,
    pub player: LocalPlayer,
    pub world: World,
    pub window: Window,
}

impl Minecraft {
    pub fn instance() -> &'static Minecraft {
        static INSTANCE: OnceLock<Arc<Minecraft>> = OnceLock::new();

        INSTANCE.get_or_init(|| unsafe {
            Arc::new(Minecraft::new().unwrap_or_else(|e| {
                error!("Failed to initialize Minecraft: {:?}", e);
                panic!("Failed to initialize Minecraft");
            }))
        })
    }

    unsafe fn new() -> anyhow::Result<Minecraft> {
        let mapping = Mapping::new()?;
        let minecraft = mapping
            .call_static_method(MinecraftClassType::Minecraft, "getInstance", &[])?
            .l()?;

        if minecraft.is_null() {
            error!("Minecraft is null")
        }

        let minecraft = mapping.new_global_ref(minecraft)?;

        let player = LocalPlayer::new(&minecraft, &mapping)?;
        let world = World::new(&minecraft, &mapping)?;
        let window = Window::new(&minecraft, &mapping)?;

        Ok(Minecraft {
            jni_ref: minecraft,
            mapping,
            player,
            world,
            window,
        })
    }

    pub fn get_mapping(&self) -> &Mapping {
        &self.mapping
    }
}

impl Deref for Minecraft {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
