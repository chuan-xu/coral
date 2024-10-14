use coral_macro::env_assign_basic;

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
env_assign_basic!(usize);
env_assign_basic!(bool);

impl<T: EnvAssignToml> EnvAssignToml for Option<T> {
    fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), serde_json::Error> {
        if let Some(this) = self {
            this.assign(prefix)?;
        }
        Ok(())
    }
}

impl EnvAssignToml for String {
    fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), serde_json::Error> {
        if let Some(prefix) = prefix {
            if let Ok(v) = std::env::var(prefix) {
                *self = v;
            }
        }
        Ok(())
    }
}

impl<T: serde::de::DeserializeOwned> EnvAssignToml for Vec<T> {
    fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), serde_json::Error> {
        if let Some(prefix) = prefix {
            if let Ok(v) = std::env::var(prefix) {
                *self = serde_json::from_str(&v)?;
            }
        }
        Ok(())
    }
}

/// Assign values using environment variables, Commonly use `coral_macro::EnvAssign`
///
/// # Example
///
/// ```rust
/// let mut a = 11;
/// std::env::set_var("CORAL_A", "13");
/// a.assign(Some("CORAL_A")).unwrap();
/// assert_eq!(a, 13);
/// ```
pub trait EnvAssignToml {
    fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), serde_json::Error>;
}
