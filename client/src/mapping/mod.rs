use crate::client::DarkClient;
use crate::mapping::class::MinecraftClass;
use crate::mapping::class_type::MinecraftClassType;
use crate::mapping::client::minecraft::Minecraft;
use crate::LogExpect;
use jni::objects::{GlobalRef, JObject, JString, JValue, JValueOwned};
use jni::JNIEnv;
use serde::Deserialize;
use std::collections::HashMap;

pub mod class;
pub mod class_type;
pub mod client;
pub mod entity;
pub mod java;

pub trait GameContext {
    fn client(&self) -> &'static DarkClient {
        DarkClient::instance()
    }

    fn minecraft(&self) -> &'static Minecraft {
        Minecraft::instance()
    }

    fn mapping(&self) -> &'static Mapping {
        self.minecraft().get_mapping()
    }
}

/// Root structure containing all mapped Minecraft classes
#[derive(Debug, Deserialize)]
pub struct Mapping {
    classes: HashMap<String, MinecraftClass>,
}

#[allow(dead_code)]
pub enum FieldType<'local> {
    Boolean,
    Byte,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    String,
    Object(MinecraftClassType, &'local Mapping),
}

impl FieldType<'_> {
    pub fn get_signature(&self) -> String {
        match self {
            FieldType::Boolean => String::from("Z"),
            FieldType::Byte => String::from("B"),
            FieldType::Char => String::from("C"),
            FieldType::Short => String::from("S"),
            FieldType::Int => String::from("I"),
            FieldType::Long => String::from("J"),
            FieldType::Float => String::from("F"),
            FieldType::Double => String::from("D"),
            FieldType::String => String::from("Ljava/lang/String;"),
            FieldType::Object(minecraft_class_type, mapping) => {
                let class_name = &mapping.get_class(minecraft_class_type.get_name()).name;
                format!("L{};", class_name)
            }
        }
    }
}

impl Mapping {
    pub fn new() -> Self {
        let contents = include_str!("../../../mappings.json");
        let mapping: Mapping = serde_json::from_str(contents).log_expect("Failed to parse mapping");
        mapping
    }

    fn get_client(&self) -> &DarkClient {
        DarkClient::instance()
    }

    fn get_env(&'_ self) -> JNIEnv<'_> {
        self.get_client()
            .get_env()
            .log_expect("Failed to get jni env")
    }

    pub fn get_class(&self, name: &str) -> &MinecraftClass {
        self.classes
            .get(name)
            .log_expect(format!("{} java class not found", name))
    }

    pub fn call_static_method(
        &'_ self,
        class_type: MinecraftClassType,
        method_name: &str,
        args: &[JValue],
    ) -> JValueOwned<'_> {
        let mut env = self.get_env();

        let class = self.get_class(class_type.get_name());
        let jclass = env
            .find_class(&class.name)
            .log_expect(format!("{} class not found", class_type.get_name()));
        let method = class.get_method_by_args(method_name, args);
        env.call_static_method(jclass, &method.name, &method.signature, args)
            .log_expect(format!(
                "Error when calling static method {} in class {} with method signature {}",
                method.name, class.name, method.signature
            ))
    }

    pub fn call_method(
        &'_ self,
        class_type: MinecraftClassType,
        instance: &JObject,
        method_name: &str,
        args: &[JValue],
    ) -> JValueOwned<'_> {
        let mut env = self.get_env();

        let class = self.get_class(class_type.get_name());
        let method = class.get_method_by_args(method_name, args);
        env.call_method(instance, &method.name, &method.signature, args)
            .log_expect(format!(
                "Error when calling method {} in class {} with method signature {}",
                method.name, class.name, method.signature
            ))
    }

    pub fn get_static_field(
        &'_ self,
        class_type: MinecraftClassType,
        field_name: &str,
        field_type: FieldType,
    ) -> JValueOwned<'_> {
        let mut env = self.get_env();

        let class = self.get_class(class_type.get_name());
        let jclass = env
            .find_class(&class.name)
            .log_expect(format!("{} class not found", class_type.get_name()));
        let field = class.get_field(field_name);
        env.get_static_field(jclass, &field.name, field_type.get_signature())
            .log_expect(format!("Error when getting static field {}", field.name))
    }

    pub fn get_field(
        &'_ self,
        class_type: MinecraftClassType,
        instance: &JObject,
        field_name: &str,
        field_type: FieldType,
    ) -> JValueOwned<'_> {
        let mut env = self.get_env();

        let class = self.get_class(class_type.get_name());
        let field = class.get_field(field_name);

        env.get_field(instance, &field.name, field_type.get_signature())
            .log_expect(format!("Error when getting field {}", field.name))
    }

    pub fn set_field(
        &self,
        class_type: MinecraftClassType,
        instance: &JObject,
        field_name: &str,
        field_type: FieldType,
        value: JValue,
    ) {
        let mut env = self.get_env();

        let class = self.get_class(class_type.get_name());
        let field = class.get_field(field_name);
        env.set_field(instance, &field.name, field_type.get_signature(), value)
            .log_expect(format!("Error when setting field {}", field.name));
    }

    pub fn new_global_ref(&self, obj: JObject) -> GlobalRef {
        let env = self.get_env();
        env.new_global_ref(obj).unwrap()
    }

    pub fn get_string(&self, obj: JObject) -> String {
        let env = self.get_env();
        let jstring = JString::from(obj);
        unsafe {
            let value = env
                .get_string_unchecked(jstring.as_ref())
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            value
        }
    }
}

impl Default for Mapping {
    fn default() -> Self {
        Self::new()
    }
}
