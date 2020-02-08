use crate::actions::exec::ExecAction;
use linked_hash_map::LinkedHashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadAction {
    into: String,
    #[serde(flatten)]
    options: LoadOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    render: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoadOptions {
    #[serde(rename = "file")]
    File(String),
    #[serde(rename = "http")]
    Http {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<LinkedHashMap<String, String>>,
    },
    #[serde(rename = "exec")]
    Exec(ExecAction),
    #[serde(rename = "inline")]
    Inline(String),
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use serde_json;
    use serde_yaml;

    use crate::actions::exec::ExecAction;
    use crate::actions::load::{LoadAction, LoadOptions};

    #[test]
    fn test_serialize_from_file() {
        let action = LoadAction {
            into: "schema".to_string(),
            options: LoadOptions::File("{{ archetype.local_path }}/schema.json".to_owned()),
            render: Some(false),
        };

        let yaml = serde_yaml::to_string(&action).unwrap();
        println!("{}", yaml);
    }

    #[test]
    fn test_serialize_from_http() {
        let action = LoadAction {
            into: "schema".to_string(),
            options: LoadOptions::Http {
                url: "http://www.example.com/schema".to_owned(),
                headers: None,
            },
            render: None,
        };

        let yaml = serde_yaml::to_string(&action).unwrap();
        println!("{}", yaml);
    }

    #[test]
    fn test_serialize_inline() {
        let action = LoadAction {
            into: "schema".to_string(),
            options: LoadOptions::Inline(
                indoc!(
                    r#"
                {
                  "into": "schema",
                  "exec": {
                    "command": "python",
                    "args": [
                      "read_schema.py"
                    ]
                  },
                  "render": true
                }
            "#
                )
                .to_string(),
            ),
            render: None,
        };

        let yaml = serde_yaml::to_string(&action).unwrap();
        println!("{}", yaml);
    }

    #[test]
    fn test_deserialize_inline() {
        let yaml = indoc!(
            r#"
            ---
            into: schema
            inline: |
                {
                  "into": "schema",
                  "exec": {
                    "command": "python",
                    "args": [
                      "read_schema.py"
                    ]
                  },
                  "render": true
                }   
        "#
        );

        let action: LoadAction = serde_yaml::from_str(&yaml).unwrap();
        if let LoadOptions::Inline(json) = action.options {
            println!("{}", json);
        }
    }

    #[test]
    fn test_serialize_from_exec() {
        let action = LoadAction {
            into: "schema".to_string(),
            options: LoadOptions::Exec(ExecAction::new("python").with_arg("read_schema.py")),
            render: Some(true),
        };

        let yaml = serde_yaml::to_string(&action).unwrap();

        println!("{}", yaml);

        let json = serde_json::to_string_pretty(&action).unwrap();
        println!("{}", json);
    }
}
