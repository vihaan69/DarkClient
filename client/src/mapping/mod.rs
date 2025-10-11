use crate::client::DarkClient;
use crate::mapping::class::MinecraftClass;
use crate::mapping::class_type::MinecraftClassType;
use crate::mapping::client::minecraft::Minecraft;
use crate::mapping::minecraft_version::MinecraftVersion;
use jni::objects::{GlobalRef, JObject, JString, JValue, JValueOwned};
use jni::JNIEnv;
use log::error;
use serde::Deserialize;
use std::collections::HashMap;

pub mod class;
pub mod class_type;
pub mod client;
pub mod entity;
pub mod java;
mod method;
mod minecraft_version;

pub trait GameContext {
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
    version: MinecraftVersion,
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
    pub fn get_signature(&self) -> anyhow::Result<String> {
        Ok(match self {
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
                let class_name = &mapping.get_class(minecraft_class_type.get_name())?.name;
                format!("L{};", class_name)
            }
        })
    }
}

#[allow(dead_code)]
impl Mapping {
    pub fn new() -> anyhow::Result<Mapping> {
        let contents = include_str!("../../../mappings.json");
        let mapping: Mapping = serde_json::from_str(contents)?;
        Ok(mapping)
    }

    fn get_client(&self) -> &DarkClient {
        DarkClient::instance()
    }

    fn get_env(&'_ self) -> anyhow::Result<JNIEnv<'_>> {
        Ok(self.get_client().get_env()?)
    }

    pub fn get_version(&self) -> MinecraftVersion {
        self.version
    }

    pub fn get_class(&self, name: &str) -> anyhow::Result<&MinecraftClass> {
        match self.classes.get(name) {
            Some(class) => Ok(class),
            None => Err(anyhow::anyhow!("{} java class not found", name)),
        }
    }

    /// Find the real name of a class given his obfuscated name
    fn find_class_by_obfuscated_name(&self, obfuscated_name: &str) -> Option<&str> {
        self.classes
            .iter()
            .find(|(_, class_data)| class_data.name == obfuscated_name)
            .map(|(deobfuscated_name, _)| deobfuscated_name.as_str())
    }

    fn translate_type_descriptor<'a>(&self, descriptor: &mut &'a str) -> String {
        let mut array_brackets = String::new();
        while descriptor.starts_with('[') {
            array_brackets.push_str("[]");
            *descriptor = &descriptor[1..];
        }

        let type_name = if let Some(stripped) = descriptor.strip_prefix('L') {
            if let Some(end_index) = stripped.find(';') {
                let obfuscated_name = &stripped[..end_index];
                let deobfuscated_name = self
                    .find_class_by_obfuscated_name(obfuscated_name)
                    .unwrap_or(obfuscated_name);

                *descriptor = &stripped[end_index + 1..];
                deobfuscated_name.to_string()
            } else {
                // Malformed, return the rest of the string
                let rest = descriptor.to_string();
                *descriptor = "";
                rest
            }
        } else {
            let (primitive, rest) = descriptor.split_at(1);
            *descriptor = rest;
            match primitive {
                "Z" => "boolean".to_string(),
                "B" => "byte".to_string(),
                "C" => "char".to_string(),
                "S" => "short".to_string(),
                "I" => "int".to_string(),
                "J" => "long".to_string(),
                "F" => "float".to_string(),
                "D" => "double".to_string(),
                "V" => "void".to_string(),
                _ => primitive.to_string(),
            }
        };

        format!("{}{}", type_name, array_brackets)
    }

    fn translate_signature(&self, signature: &str) -> String {
        if let (Some(params_start), Some(params_end)) = (signature.find('('), signature.find(')')) {
            let mut params_str = &signature[params_start + 1..params_end];
            let mut return_type_str = &signature[params_end + 1..];

            let mut translated_params = Vec::new();
            while !params_str.is_empty() {
                translated_params.push(self.translate_type_descriptor(&mut params_str));
            }

            let translated_return = self.translate_type_descriptor(&mut return_type_str);

            format!("({}) -> {}", translated_params.join(", "), translated_return)
        } else {
            signature.to_string() // Return the original signature if it's not a valid signature
        }
    }

    pub fn call_static_method(
        &'_ self,
        class_type: MinecraftClassType,
        method_name: &str,
        args: &[JValue],
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class = self.get_class(class_type.get_name())?;
        let jclass = match env.find_class(&class.name) {
            Ok(jclass) => jclass,
            Err(_) => return Err(anyhow::anyhow!("Class {} ({}) not found", class_type.get_name(), class.name)),
        };
        let method = class.get_method_by_args(method_name, args)?;
        match env.call_static_method(jclass, &method.name, &method.signature, args) {
            Ok(value) => Ok(value),
            Err(_) => {
                let translated_signature = self.translate_signature(&method.signature);
                Err(anyhow::anyhow!(
                    "Error calling static method {} ({}) in class {} ({}) with signature {} ({})",
                    method_name,
                    method.name,
                    class_type.get_name(),
                    class.name,
                    translated_signature,
                    method.signature
                ))
            }
        }
    }

    pub fn call_method(
        &'_ self,
        class_type: MinecraftClassType,
        instance: &JObject,
        method_name: &str,
        args: &[JValue],
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class = self.get_class(class_type.get_name())?;
        let method = class.get_method_by_args(method_name, args)?;
        match env.call_method(instance, &method.name, &method.signature, args) {
            Ok(value) => Ok(value),
            Err(_) => {
                let translated_signature = self.translate_signature(&method.signature);
                Err(anyhow::anyhow!(
                    "Error calling method {} ({}) in class {} ({}) with signature {} ({})",
                    method_name,
                    method.name,
                    class_type.get_name(),
                    class.name,
                    translated_signature,
                    method.signature
                ))
            }
        }
    }

    pub fn get_static_field(
        &'_ self,
        class_type: MinecraftClassType,
        field_name: &str,
        field_type: FieldType,
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class = self.get_class(class_type.get_name())?;
        let jclass = match env.find_class(&class.name) {
            Ok(jclass) => jclass,
            Err(_) => return Err(anyhow::anyhow!("Class {} ({}) not found", class_type.get_name(), class.name)),
        };
        let field = class.get_field(field_name)?;
        match env.get_static_field(jclass, &field.name, field_type.get_signature()?) {
            Ok(value) => Ok(value),
            Err(_) => {
                Err(anyhow::anyhow!(
                    "Error getting static field {} ({}) from class {} ({})",
                    field_name,
                    field.name,
                    class_type.get_name(),
                    class.name
                ))
            }
        }
    }

    pub fn get_field(
        &'_ self,
        class_type: MinecraftClassType,
        instance: &JObject,
        field_name: &str,
        field_type: FieldType,
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class = self.get_class(class_type.get_name())?;
        let field = class.get_field(field_name)?;

        match env.get_field(instance, &field.name, field_type.get_signature()?) {
            Ok(value) => Ok(value),
            Err(_) => {
                Err(anyhow::anyhow!(
                    "Error getting field {} ({}) from class {} ({})",
                    field_name,
                    field.name,
                    class_type.get_name(),
                    class.name
                ))
            }
        }
    }

    pub fn set_field(
        &self,
        class_type: MinecraftClassType,
        instance: &JObject,
        field_name: &str,
        field_type: FieldType,
        value: JValue,
    ) -> anyhow::Result<()> {
        let mut env = self.get_env()?;

        let class = self.get_class(class_type.get_name())?;
        let field = class.get_field(field_name)?;
        match env.set_field(instance, &field.name, field_type.get_signature()?, value) {
            Ok(_) => Ok(()),
            Err(_) => {
                Err(anyhow::anyhow!(
                    "Error setting field {} ({}) in class {} ({})",
                    field_name,
                    field.name,
                    class_type.get_name(),
                    class.name
                ))
            }
        }
    }

    pub fn new_global_ref(&self, obj: JObject) -> anyhow::Result<GlobalRef> {
        let env = self.get_env()?;
        Ok(env.new_global_ref(obj)?)
    }

    pub fn get_string(&self, obj: JObject) -> anyhow::Result<String> {
        let env = self.get_env()?;
        let jstring = JString::from(obj);
        unsafe {
            let value = env
                .get_string_unchecked(jstring.as_ref())?
                .to_str()?
                .to_string();
            Ok(value)
        }
    }
}

impl Default for Mapping {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            error!("Failed to load mappings");
            panic!("Failed to load mappings");
        })
    }
}