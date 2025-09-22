use crate::mapping::{GameContext, Mapping, MinecraftClassType};
use jni::objects::GlobalRef;
use jni::sys::jlong;
use std::ops::Deref;

#[derive(Debug)]
pub struct Window {
    pub jni_ref: GlobalRef,
}

impl GameContext for Window {}

impl Window {
    pub fn new(minecraft: &GlobalRef, mapping: &Mapping) -> anyhow::Result<Window> {
        let window_obj = mapping
            .call_method(
                MinecraftClassType::Minecraft,
                minecraft.as_obj(),
                "getWindow",
                &[],
            )?
            .l()?;

        Ok(Window {
            jni_ref: mapping.new_global_ref(window_obj)?,
        })
    }

    pub fn get_window(&self) -> anyhow::Result<jlong> {
        let mapping = self.mapping();

        Ok(mapping
            .call_method(
                MinecraftClassType::Window,
                self.jni_ref.as_obj(),
                "getWindow",
                &[],
            )?
            .j()?)
    }
}

impl Deref for Window {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
