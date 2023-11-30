use std::collections::HashSet;
use camino::{Utf8Path, Utf8PathBuf};
use rhai::Map;

#[derive(Clone)]
pub struct RenderContext {
    destination: Utf8PathBuf,
    answers: Map,
    switches: HashSet<String>,
    settings: Map,
}

impl RenderContext {
    pub fn new<T: Into<Utf8PathBuf>>(destination: T, answers: Map) -> RenderContext {
        RenderContext {
            destination: destination.into(),
            answers,
            switches: Default::default(),
            settings: Default::default(),
        }
    }

    pub fn answers(&self) -> &Map {
        &self.answers
    }

    pub fn answers_owned(&self) -> Map {
        self.answers.clone()
    }

    pub fn destination(&self) -> &Utf8Path {
        self.destination.as_path()
    }

    pub fn switches(&self) -> &HashSet<String> {
        &self.switches
    }

    pub fn with_switches(mut self, switches: HashSet<String>) -> Self {
        self.switches = switches;
        self
    }

    pub fn settings(&self) -> &Map {
        &self.settings
    }

    pub fn with_settings(mut self, settings: Map) -> Self {
        self.settings = settings;
        self
    }
}

// fn create_owned_map(input: &Map) -> Map {
//     let mut results = Map::new();
//     results.extend(input.clone());
//     results
// }

#[cfg(test)]
mod tests {
    //     #[test]
    //     fn test_from_rhai() {
    //         let map = r#"
    // #{
    //     first_name: "Jimmie",
    //     last_name: "Fulton",
    // }
    //         "#;
    //
    //         let list = r#"
    // ["Jimmie", "Shirley", "Bailey"]
    //          "#;
    //
    //         let string = r#"
    //             "Jimmie"
    //         "#;
    //
    //         let int = r#"
    //             24
    //         "#;
    //
    //         let engine = Engine::new();
    //         let result: Dynamic = engine.eval::<Dynamic>(int).unwrap();
    //         println!("{:?}", result.type_name());
    //     }

    //     #[test]
    //     fn test_from_json() {
    //         let map = r#"
    // {
    //     "first_name": "Jimmie",
    //     "last_name": "Fulton"
    // }
    //         "#;
    //
    //         let list = r#"
    // ["Jimmie", "Shirley", "Bailey"]
    //          "#;
    //
    //         let string = r#"
    //             "Jimmie"
    //         "#;
    //
    //         let int = r#"
    //             24
    //         "#;
    //
    //         let result: Dynamic = serde_json::from_str(map).unwrap();
    //         // match result {
    //         //     Value::Null => println!("Null"),
    //         //     Value::Bool(value) => println!("Bool: {value:?}"),
    //         //     Value::Number(value) =>  println!("Number: {value:?}"),
    //         //     Value::String(value) =>  println!("String: {value:?}"),
    //         //     Value::Array(value) =>  println!("Array: {value:?}"),
    //         //     Value::Object(value) =>  println!("Object: {value:?}"),
    //         // }
    //         println!("{}", result.type_name());
    //         println!("{}", serde_json::to_string(&result).unwrap());
    //     }
}
