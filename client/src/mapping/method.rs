use crate::mapping::minecraft_version::MinecraftVersion;

#[derive(Debug, Clone, Copy)]
pub enum MethodName {
    WindowGetWindow,
}

impl MethodName {
    pub fn get_name(&self, minecraft_version: MinecraftVersion) -> &str {
        match self {
            Self::WindowGetWindow => {
                if minecraft_version < MinecraftVersion::new(1, 21, 9) {
                    "getWindow"
                } else {
                    "handle"
                }
            }
        }
    }
}
