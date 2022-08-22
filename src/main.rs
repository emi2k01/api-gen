#![allow(unused)]

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;

use eyre::{Context, Result};
use indoc::indoc;
use serde::{Deserialize, Serialize};
use string_template::Template;

#[derive(Copy, Clone, PartialEq, Eq, Deserialize)]
enum ApiDocsModelObjectType {
    String,
    Number,
    Boolean,
    Object,
    Array,
    Enum,
}

type ApiDocsModelObject = BTreeMap<String, ApiDocsModel>;
type ApiDocsModelsObject = BTreeMap<String, ApiDocsModel>;

#[derive(Deserialize)]
struct ApiDocsModel {
    r#type: ApiDocsModelObjectType,
    /// Model if `type` is `object`
    fields: Option<ApiDocsModelObject>,
    /// Model if `type` is `array`
    model: Option<Box<ApiDocsModel>>,
    /// Model if `type` is `enum`
    members: Option<Vec<serde_json::Value>>,
    required: bool,
}

#[derive(Deserialize)]
struct ApiDocsRoute {
    accepts: String,
    returns: String,
}

#[derive(Deserialize)]
struct ApiDocs {
    models: BTreeMap<String, ApiDocsModelsObject>,
    routes: BTreeMap<String, ApiDocsRoute>,
}

struct Args {
    file: String,
    out: String,
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

fn render_field_type(obj: &ApiDocsModel) -> String {
    let inner_type = match &obj.r#type {
        ApiDocsModelObjectType::String => "string".to_string(),
        ApiDocsModelObjectType::Number => "number".to_string(),
        ApiDocsModelObjectType::Boolean => "boolean".to_string(),
        ApiDocsModelObjectType::Array => {
            format!(
                "Array<{}>",
                render_field_type(
                    obj.model
                        .as_ref()
                        .expect("`model` must be present if `type` is `\"array\"`")
                )
            )
        },
        ApiDocsModelObjectType::Object => {
            format!(
                "{{ {} }}",
                render_fields(
                    obj.fields
                        .as_ref()
                        .expect("`fields` must be set if `type` is `\"object\"`.")
                )
            )
        },
        _ => todo!(),
    };

    if !obj.required {
        format!("Optional<{inner_type}>")
    } else {
        inner_type
    }
}

fn render_field(name: &str, model: &ApiDocsModel) -> String {
    format!(
        "{name}{opt}: {type},",
        opt = model.required.then_some("").unwrap_or("?"),
        r#type = render_field_type(model)
    )
}

fn render_fields(obj: &ApiDocsModelsObject) -> String {
    obj.iter()
        .map(|(name, model)| render_field(name, model))
        .collect::<String>()
}

fn render_interface(name: &str, obj: &ApiDocsModelObject) -> String {
    format!("interface {name} {{ {} }}", render_fields(obj))
}

fn render_interfaces(models: &BTreeMap<String, ApiDocsModelObject>) -> String {
    models
        .iter()
        .map(|(model_name, model)| {
            let name = heck::AsPascalCase(model_name).to_string();
            render_interface(&name, model)
        })
        .collect::<String>()
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut args = pico_args::Arguments::from_env();
    let args = Args {
        file: args.value_from_str("--file")?,
        out: args.value_from_str("--out")?,
    };

    let api_docs: ApiDocs = serde_json::from_reader(
        File::open(&args.file)
            .wrap_err_with(|| format!("Failed to open: {}", args.file.clone()))?,
    )?;

    let interfaces = render_interfaces(&api_docs.models);

    let mut out_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&args.out)?;

    out_file.write_all(interfaces.as_bytes());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_field_type_string() {
        let rendered = render_field_type(&ApiDocsModel {
            r#type: ApiDocsModelObjectType::String,
            fields: None,
            model: None,
            members: None,
            required: true,
        });
        k9::snapshot!(rendered, "string");
    }

    #[test]
    fn test_render_field_type_number() {
        let rendered = render_field_type(&ApiDocsModel {
            r#type: ApiDocsModelObjectType::Number,
            fields: None,
            model: None,
            members: None,
            required: false,
        });
        k9::snapshot!(rendered, "Optional<number>");
    }

    #[test]
    fn test_render_field_type_boolean() {
        let rendered = render_field_type(&ApiDocsModel {
            r#type: ApiDocsModelObjectType::Boolean,
            fields: None,
            model: None,
            members: None,
            required: false,
        });
        k9::snapshot!(rendered, "Optional<boolean>");
    }

    #[test]
    fn test_render_field_type_array_of_scalar() {
        let rendered = render_field_type(&ApiDocsModel {
            r#type: ApiDocsModelObjectType::Array,
            fields: None,
            members: None,
            model: Some(Box::new(ApiDocsModel {
                r#type: ApiDocsModelObjectType::Boolean,
                fields: None,
                model: None,
                members: None,
                required: true,
            })),
            required: true,
        });
        k9::snapshot!(rendered, "Array<boolean>");
    }

    #[test]
    fn test_render_field_type_array_of_object() {
        let rendered = render_field_type(&ApiDocsModel {
            r#type: ApiDocsModelObjectType::Array,
            fields: None,
            members: None,
            model: Some(Box::new(ApiDocsModel {
                r#type: ApiDocsModelObjectType::Object,
                members: None,
                model: None,
                fields: Some(
                    [
                        (
                            "foo".to_string(),
                            ApiDocsModel {
                                r#type: ApiDocsModelObjectType::String,
                                fields: None,
                                members: None,
                                model: None,
                                required: true,
                            },
                        ),
                        (
                            "bar".to_string(),
                            ApiDocsModel {
                                r#type: ApiDocsModelObjectType::Boolean,
                                fields: None,
                                members: None,
                                model: None,
                                required: true,
                            },
                        ),
                    ]
                    .into(),
                ),
                required: true,
            })),
            required: false,
        });
        k9::snapshot!(rendered, "Optional<Array<{ bar: boolean,foo: string, }>>");
    }

    #[test]
    fn test_render_field_type_array_of_array() {
        let rendered = render_field_type(&ApiDocsModel {
            r#type: ApiDocsModelObjectType::Array,
            fields: None,
            members: None,
            model: Some(Box::new(ApiDocsModel {
                r#type: ApiDocsModelObjectType::Array,
                fields: None,
                members: None,
                model: Some(Box::new(ApiDocsModel {
                    r#type: ApiDocsModelObjectType::String,
                    fields: None,
                    members: None,
                    model: None,
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
        let rendered = render_field(
            "foo",
            &ApiDocsModel {
                r#type: ApiDocsModelObjectType::Boolean,
                fields: None,
                model: None,
                members: None,
                required: true,
            },
        );
        k9::snapshot!(rendered, "foo: boolean,");
    }

    #[test]
    fn test_render_non_required_field() {
        let rendered = render_field(
            "foo",
            &ApiDocsModel {
                r#type: ApiDocsModelObjectType::Boolean,
                fields: None,
                model: None,
                members: None,
                required: false,
            },
        );
        k9::snapshot!(rendered, "foo?: Optional<boolean>,");
    }

    #[test]
    fn test_render_interface_simple() {
        let rendered = render_interface(
            "Foo",
            &[
                (
                    "foo".to_string(),
                    ApiDocsModel {
                        r#type: ApiDocsModelObjectType::String,
                        fields: None,
                        members: None,
                        model: None,
                        required: true,
                    },
                ),
                (
                    "bar".to_string(),
                    ApiDocsModel {
                        r#type: ApiDocsModelObjectType::Boolean,
                        fields: None,
                        members: None,
                        model: None,
                        required: true,
                    },
                ),
            ]
            .into(),
        );
        k9::snapshot!(rendered, "interface Foo { bar: boolean,foo: string, }");
    }

    #[test]
    fn test_render_interface_with_nested_objects() {
        let rendered = render_interface(
            "Foo",
            &[
                (
                    "foo".to_string(),
                    ApiDocsModel {
                        r#type: ApiDocsModelObjectType::String,
                        fields: None,
                        members: None,
                        model: None,
                        required: true,
                    },
                ),
                (
                    "bar".to_string(),
                    ApiDocsModel {
                        r#type: ApiDocsModelObjectType::Object,
                        fields: Some(
                            [
                                (
                                    "foo".to_string(),
                                    ApiDocsModel {
                                        r#type: ApiDocsModelObjectType::String,
                                        fields: None,
                                        members: None,
                                        model: None,
                                        required: true,
                                    },
                                ),
                                (
                                    "bar".to_string(),
                                    ApiDocsModel {
                                        r#type: ApiDocsModelObjectType::Boolean,
                                        fields: None,
                                        members: None,
                                        model: None,
                                        required: true,
                                    },
                                ),
                            ]
                            .into(),
                        ),
                        members: None,
                        model: None,
                        required: true,
                    },
                ),
            ]
            .into(),
        );
        k9::snapshot!(
            rendered,
            "interface Foo { bar: { bar: boolean,foo: string, },foo: string, }"
        );
    }

    #[test]
    fn test_render_models_simple() {
        let rendered = render_interfaces(
            &[(
                "Foo".to_string(),
                [(
                    "baz".to_string(),
                    ApiDocsModel {
                        r#type: ApiDocsModelObjectType::Boolean,
                        required: true,
                        fields: None,
                        members: None,
                        model: None,
                    },
                )]
                .into(),
            )]
            .into(),
        );
        k9::snapshot!(rendered, "interface Foo { baz: boolean, }");
    }
}
