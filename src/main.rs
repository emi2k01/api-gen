#![allow(unused)]

use eyre::{Context, Result};
use indoc::indoc;
use serde::{Deserialize, Serialize};
use string_template::Template;

use std::{collections::BTreeMap, fs::File};

#[derive(Copy, Clone, PartialEq, Eq, Deserialize)]
enum ApiDocsSchemaObjectType {
    String,
    Number,
    Boolean,
    Object,
    Array,
    Enum,
}

type ApiDocsSchemaObject = BTreeMap<String, ApiDocsSchema>;
type ApiDocsSchemasObject = BTreeMap<String, ApiDocsSchema>;

#[derive(Deserialize)]
struct ApiDocsSchema {
    r#type: ApiDocsSchemaObjectType,
    /// Schema if `type` is `object`
    fields: Option<ApiDocsSchemaObject>,
    /// Schema if `type` is `array`
    schema: Option<Box<ApiDocsSchema>>,
    /// Schema if `type` is `enum`
    members: Option<Vec<String>>,
    required: bool,
}

#[derive(Deserialize)]
struct ApiDocsRoute {
    accepts: String,
    returns: String,
}

#[derive(Deserialize)]
struct ApiDocs {
    schemas: ApiDocsSchemasObject,
    routes: BTreeMap<String, ApiDocsRoute>,
}

struct Args {
    file: String,
}

fn interface_field_template(name: &str, r#type: &str) -> String {
    Template::new(indoc! {"
        {{name}}: {{type}},
    "})
    .render(&[("name", name), ("type", r#type)].into())
}

fn interface_field_object_template(content: &str) -> String {
    Template::new(indoc! {"
        {
            {{content}}
        },
    "})
    .render(&[("content", content)].into())
}

fn interface_template(name: &str, content: &str) -> String {
    Template::new(indoc! {"
        interface {{name}} {
            {{content}}
        }
    "})
    .render(&[("name", name), ("content", content)].into())
}

fn render_field_type(obj: &ApiDocsSchema) -> String {
    let inner_type = match &obj.r#type {
        ApiDocsSchemaObjectType::String => "string".to_string(),
        ApiDocsSchemaObjectType::Number => "number".to_string(),
        ApiDocsSchemaObjectType::Boolean => "boolean".to_string(),
        ApiDocsSchemaObjectType::Array => format!(
            "Array<{}>",
            render_field_type(
                &obj.schema
                    .as_ref()
                    .expect("`schema` must be present if `type` is `\"array\"`")
            )
        ),
        ApiDocsSchemaObjectType::Object => format!(
            "{{ {} }}",
            render_interface_fields(
                &obj.fields
                    .as_ref()
                    .expect("`fields` must be set if `type` is `\"object\"`.")
            )
        ),
        _ => todo!(),
    };

    if !obj.required {
        format!("Optional<{inner_type}>")
    } else {
        inner_type
    }
}

fn render_interface_field(name: &str, schema: &ApiDocsSchema) -> String {
    format!(
        "{name}{opt}: {type},",
        opt = schema.required.then_some("").unwrap_or("?"),
        r#type = render_field_type(schema)
    )
}

fn render_interface_fields(obj: &ApiDocsSchemasObject) -> String {
    todo!()
}

fn render_interface(name: &str, obj: &ApiDocsSchema) -> String {
    todo!()
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut args = pico_args::Arguments::from_env();
    let args = Args {
        file: args.value_from_str("--file")?,
    };

    let api_docs: ApiDocs = serde_json::from_reader(
        File::open(&args.file)
            .wrap_err_with(|| format!("Failed to open: {}", args.file.clone()))?,
    )?;

    api_docs.schemas.iter().map(|(schema_name, schema_object)| {
        let name = heck::AsPascalCase(schema_name).to_string();
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_field_type_string() {
        let rendered = render_field_type(&ApiDocsSchema {
            r#type: ApiDocsSchemaObjectType::String,
            fields: None,
            schema: None,
            members: None,
            required: true,
        });
        k9::snapshot!(rendered, "string");
    }

    #[test]
    fn test_render_field_type_number() {
        let rendered = render_field_type(&ApiDocsSchema {
            r#type: ApiDocsSchemaObjectType::Number,
            fields: None,
            schema: None,
            members: None,
            required: false,
        });
        k9::snapshot!(rendered, "Optional<number>");
    }

    #[test]
    fn test_render_field_type_boolean() {
        let rendered = render_field_type(&ApiDocsSchema {
            r#type: ApiDocsSchemaObjectType::Boolean,
            fields: None,
            schema: None,
            members: None,
            required: false,
        });
        k9::snapshot!(rendered, "Optional<boolean>");
    }

    #[test]
    fn test_render_field_type_array_of_scalar() {
        let rendered = render_field_type(&ApiDocsSchema {
            r#type: ApiDocsSchemaObjectType::Array,
            fields: None,
            members: None,
            schema: Some(Box::new(ApiDocsSchema {
                r#type: ApiDocsSchemaObjectType::Boolean,
                fields: None,
                schema: None,
                members: None,
                required: true,
            })),
            required: true,
        });
        k9::snapshot!(rendered, "Array<boolean>");
    }

    #[test]
    fn test_render_field_type_array_of_array() {
        let rendered = render_field_type(&ApiDocsSchema {
            r#type: ApiDocsSchemaObjectType::Array,
            fields: None,
            members: None,
            schema: Some(Box::new(ApiDocsSchema {
                r#type: ApiDocsSchemaObjectType::Array,
                fields: None,
                members: None,
                schema: Some(Box::new(ApiDocsSchema {
                    r#type: ApiDocsSchemaObjectType::String,
                    fields: None,
                    members: None,
                    schema: None,
                    required: true,
                })),
                required: true,
            })),
            required: false,
        });
        k9::snapshot!(rendered, "Optional<Array<Array<string>>>");
    }

    #[test]
    fn test_render_required_field() {
        let rendered = render_interface_field(
            "foo",
            &ApiDocsSchema {
                r#type: ApiDocsSchemaObjectType::Boolean,
                fields: None,
                schema: None,
                members: None,
                required: true,
            },
        );
        k9::snapshot!(rendered, "foo: boolean,");
    }

    #[test]
    fn test_render_non_required_field() {
        let rendered = render_interface_field(
            "foo",
            &ApiDocsSchema {
                r#type: ApiDocsSchemaObjectType::Boolean,
                fields: None,
                schema: None,
                members: None,
                required: false,
            },
        );
        k9::snapshot!(rendered, "foo?: Optional<boolean>,");
    }
}
