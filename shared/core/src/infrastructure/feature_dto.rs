//! DTO declarations are the source for serde and the build-only TS export.
#[macro_export]
macro_rules! feature_dto {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        #[allow(non_snake_case)]
        pub struct $name { $(pub $field: $ty),* }
    };
}
pub use crate::feature_dto as dto;

/// Build-only schema marker for fields omitted by serde when None.
#[cfg(feature = "schema")]
pub struct Omitted<T>(std::marker::PhantomData<T>);
#[cfg(feature = "schema")]
impl<T: schemars::JsonSchema> schemars::JsonSchema for Omitted<T> {
    fn schema_name() -> String { T::schema_name() }
    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = T::json_schema(generator).into_object();
        schema.extensions.insert("x-serde-omitted".into(), serde_json::json!(true));
        schema.into()
    }
    fn is_referenceable() -> bool { false }
}
