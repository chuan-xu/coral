use coral_conf::EnvAssignToml;
use coral_macro::EnvAssign;
use serde::Deserialize;

#[derive(Deserialize, EnvAssign, Debug)]
enum EnumConf {
    Item1(String),
    Item2(SubStu),
}

#[derive(Deserialize, EnvAssign, Debug)]
struct SubStu {
    val: u16,
}

#[derive(Deserialize, EnvAssign, Debug)]
enum PureEnum {
    Val1,
    Val2,
    Val3,
}

#[derive(Deserialize, EnvAssign, Debug)]
struct Conf {
    item1: String,
    enum1: EnumConf,
    enum2: EnumConf,
    sub1: SubStu,
    pe: PureEnum,
}

#[test]
fn test_macro() {
    let toml_str = r#"
        item1 = ""
        pe = "Val3"
        [enum1]
        Item1 = ""
        [enum2.Item2]
        val = 0
        [sub1]
        val = 0
    "#;
    let mut conf: Conf = toml::from_str(toml_str).unwrap();
    assert!(matches!(conf.pe, PureEnum::Val3));
    std::env::set_var("CORAL_CONF_ITEM1", "item1");
    std::env::set_var("CORAL_CONF_ENUM1_ITEM1", "enum1.item1");
    std::env::set_var("CORAL_CONF_ENUM2_ITEM2_VAL", "1");
    std::env::set_var("CORAL_CONF_SUB1_VAL", "2");
    std::env::set_var("CORAL_CONF_PE", "Val1");
    conf.assign(Some("CORAL_CONF")).unwrap();
    assert_eq!(conf.item1, "item1");
    assert!(matches!(conf.enum1, EnumConf::Item1(x) if x == "enum1.item1"));
    assert!(matches!(conf.enum2, EnumConf::Item2(x) if x.val == 1));
    assert_eq!(conf.sub1.val, 2);
    assert!(matches!(conf.pe, PureEnum::Val1));
}

#[test]
fn test_env_assign() {
    let mut a = 11;
    std::env::set_var("CORAL_A", "13");
    a.assign(Some("CORAL_A")).unwrap();
    assert_eq!(a, 13);
}
