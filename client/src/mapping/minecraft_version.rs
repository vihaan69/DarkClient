use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MinecraftVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl MinecraftVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> MinecraftVersion {
        MinecraftVersion {
            major,
            minor,
            patch,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl<'de> Deserialize<'de> for MinecraftVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() != 3 {
            return Err(serde::de::Error::custom(format!(
                "Invalid version format: {} (expected major.minor.patch)",
                s
            )));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| serde::de::Error::custom("Invalid major version"))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| serde::de::Error::custom("Invalid minor version"))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| serde::de::Error::custom("Invalid patch version"))?;

        Ok(MinecraftVersion {
            major,
            minor,
            patch,
        })
    }
}
