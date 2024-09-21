use coral_macro::env_assign_basic;
use coral_macro::EnvAssign;
use serde::Deserialize;
use toml::from_str;
env_assign_basic!(u8);
env_assign_basic!(u16);
env_assign_basic!(u32);
env_assign_basic!(u64);
env_assign_basic!(i8);
env_assign_basic!(i16);
env_assign_basic!(i32);
env_assign_basic!(i64);
env_assign_basic!(f32);
env_assign_basic!(f64);

// #[derive(EnvAssign, Deserialize, Debug)]
#[derive(EnvAssign)]
struct Tls {
    name: String,
    age: u8,
    val: Vec<i32>,
}

impl<T: serde::de::DeserializeOwned> EnvAssignToml for Vec<T> {
    fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), toml::de::Error> {
        if let Some(prefix) = prefix {
            if let Ok(v) = std::env::var(prefix) {
                *self = toml::from_str(&v)?;
            }
        }
        Ok(())
    }
}

pub trait EnvAssignToml {
    fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), toml::de::Error>;
}
