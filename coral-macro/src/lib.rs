use proc_macro::TokenStream;

mod env_assign;
mod trace_log;

#[proc_macro]
pub fn env_assign_basic(input: TokenStream) -> TokenStream {
    env_assign::assign_basic(input)
}

///
/// # Example
/// ```

/// #[derive(Deserialize, EnvAssign, Debug)]
/// enum EnumConf {
///     Item1(String),
///     Item2(SubStu),
/// }
///
/// #[derive(Deserialize, EnvAssign, Debug)]
/// struct SubStu {
///     val: u16,
/// }
///
/// #[derive(Deserialize, EnvAssign, Debug)]
/// enum PureEnum {
///     Val1,
///     Val2,
///     Val3,
/// }
///
/// #[derive(Deserialize, EnvAssign, Debug)]
/// struct Conf {
///     item1: String,
///     enum1: EnumConf,
///     enum2: EnumConf,
///     sub1: SubStu,
///     pe: PureEnum,
/// }
///
/// fn check() {
///     let toml_str = r#"
///         item1 = ""
///         [enum1]
///         Item1 = ""
///         [enum2.Item2]
///         val = 0
///         [sub1]
///         val = 0
///     "#;
///     let mut conf: Conf = toml::from_str(toml_str).unwrap();
///     std::env::set_var("CORAL_CONF_ITEM1", "item1");
///     std::env::set_var("CORAL_CONF_ENUM1_ITEM1", "enum1.item1");
///     std::env::set_var("CORAL_CONF_ENUM2_ITEM2_VAL", "1");
///     std::env::set_var("CORAL_CONF_SUB1_VAL", "2");
///     conf.assign(Some("CORAL_CONF")).unwrap();
///     assert_eq!(conf.item1, "item1");
///     assert!(matches!(conf.enum1, EnumConf::Item1(x) if x == "enum1.item1"));
///     assert!(matches!(conf.enum2, EnumConf::Item2(x) if x.val == 1));
///     assert_eq!(conf.sub1.val, 2);
/// }
/// ```
#[proc_macro_derive(EnvAssign)]
pub fn derive(input: TokenStream) -> TokenStream {
    env_assign::assign_struct(input)
}

#[proc_macro]
pub fn trace_error(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Error)
}
#[proc_macro]
pub fn trace_warn(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Warn)
}
#[proc_macro]
pub fn trace_info(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Info)
}
#[proc_macro]
pub fn trace_debug(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Debug)
}
#[proc_macro]
pub fn trace_trace(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Trace)
}
