use crate::mapping::entity::Entity;
use crate::mapping::{FieldType, GameContext, Mapping, MinecraftClassType};
use jni::objects::{GlobalRef, JValue};
use jni::sys::jboolean;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct LocalPlayer {
    pub jni_ref: GlobalRef,
    pub abilities: Abilities,
    pub entity: Entity,
}

#[derive(Debug, Clone)]
pub struct Abilities {
    pub jni_ref: GlobalRef,
}

impl GameContext for LocalPlayer {}
impl GameContext for Abilities {}

impl LocalPlayer {
    pub fn new(minecraft: &GlobalRef, mapping: &Mapping) -> anyhow::Result<Self> {
        let player_obj = mapping
            .get_field(
                MinecraftClassType::Minecraft,
                minecraft.as_obj(),
                "player",
                FieldType::Object(MinecraftClassType::LocalPlayer, mapping),
            )?
            .l()?;

        let player_ref = mapping.new_global_ref(player_obj)?;
        let abilities = Abilities::new(player_ref.clone(), mapping)?;
        let entity = Entity::new(player_ref.clone());

        Ok(Self {
            jni_ref: player_ref,
            abilities,
            entity,
        })
    }
}

impl Abilities {
    pub fn new(player: GlobalRef, mapping: &Mapping) -> anyhow::Result<Self> {
        let jni_ref = mapping
            .call_method(MinecraftClassType::Player, &player, "getAbilities", &[])?
            .l()?;
        Ok(Self {
            jni_ref: mapping.new_global_ref(jni_ref)?,
        })
    }

    pub fn fly(&self, value: bool) -> anyhow::Result<()> {
        let mapping = self.mapping();

        let value: jboolean = if value { 1 } else { 0 };

        mapping.set_field(
            MinecraftClassType::Abilities,
            self.jni_ref.as_obj(),
            "flying",
            FieldType::Boolean,
            JValue::Bool(value),
        )?;

        mapping.set_field(
            MinecraftClassType::Abilities,
            self.jni_ref.as_obj(),
            "mayfly",
            FieldType::Boolean,
            JValue::Bool(value),
        )?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_may_fly(&self) -> anyhow::Result<bool> {
        let mapping = self.mapping();

        Ok(mapping
            .get_field(
                MinecraftClassType::Abilities,
                self.jni_ref.as_obj(),
                "mayfly",
                FieldType::Boolean,
            )?
            .z()?)
    }
}

impl Deref for LocalPlayer {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}

impl Deref for Abilities {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
