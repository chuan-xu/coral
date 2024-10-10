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

pub trait EnvAssignToml {
    fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), serde_json::Error>;
}

mod test {
    use crate::EnvAssignToml;
    use coral_macro::EnvAssign;
    use serde::Deserialize;

    #[derive(EnvAssign, Deserialize, Debug)]
    struct H2 {
        port: u16,
        domain: String,
    }

    #[derive(EnvAssign, Deserialize, Debug)]
    struct H3 {
        port: u16,
        domain: String,
        tls: Option<TlsParam>,
    }

    #[derive(EnvAssign, Deserialize, Debug)]
    struct TlsParam {
        ca: String,
        cert: String,
        key: String,
    }

    #[derive(EnvAssign, Deserialize, Debug)]
    struct Config {
        h2: H2,
        h3: H3,
    }

    #[test]
    fn run() {
        let toml_str = r#"
            [h2]
            port = 9000
            domain = "server.test.com"

            [h3]
            port = 9001
            domain = "server.test.com"

            [h3.tls]
            ca = ""
            cert = ""
            key = ""
        "#;
        let mut conf: Config = toml::from_str(toml_str).unwrap();
        conf.assign(Some("CORAL_SERVER")).unwrap();
        println!("{:?}", conf);
    }
}
