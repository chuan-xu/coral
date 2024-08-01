// This file is @generated by prost-build.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FieldVal {
    #[prost(enumeration = "Type", tag = "1")]
    pub r#type: i32,
    #[prost(string, tag = "2")]
    pub val: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Span {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(map = "string, message", tag = "2")]
    pub fields: ::std::collections::HashMap<::prost::alloc::string::String, FieldVal>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Event {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(map = "string, message", tag = "2")]
    pub fields: ::std::collections::HashMap<::prost::alloc::string::String, FieldVal>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Record {
    #[prost(string, tag = "1")]
    pub timestamp: ::prost::alloc::string::String,
    #[prost(enumeration = "Level", tag = "2")]
    pub level: i32,
    #[prost(string, tag = "3")]
    pub thread_name: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub file: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub line: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "6")]
    pub spans: ::prost::alloc::vec::Vec<Span>,
    #[prost(message, optional, tag = "7")]
    pub span: ::core::option::Option<Span>,
    #[prost(message, optional, tag = "8")]
    pub event: ::core::option::Option<Event>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}
impl Level {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "TRACE" => Some(Self::Trace),
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Type {
    I = 0,
    F = 1,
    S = 2,
}
impl Type {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Type::I => "I",
            Type::F => "F",
            Type::S => "S",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "I" => Some(Self::I),
            "F" => Some(Self::F),
            "S" => Some(Self::S),
            _ => None,
        }
    }
}
