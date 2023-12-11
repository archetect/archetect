use camino::{Utf8Path, Utf8PathBuf};
use rhai::{Dynamic, Map};
use std::collections::HashSet;

#[derive(Clone)]
pub struct RenderContext {
    destination: Utf8PathBuf,
    answers: Map,
    defaults: HashSet<String>,
    defaults_all: bool,
    switches: HashSet<String>,
    settings: Map,
}

impl RenderContext {
    pub fn new<T: Into<Utf8PathBuf>>(destination: T, answers: Map) -> RenderContext {
        RenderContext {
            destination: destination.into(),
            answers,
            defaults: Default::default(),
            defaults_all: false,
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

    pub fn switches_as_array(&self) -> rhai::Array {
        self.switches.iter().map(|v| v.into()).collect()
    }

    pub fn with_switch<S: Into<String>>(mut self, switch: S) -> Self {
        self.switches.insert(switch.into());
        self
    }

    pub fn with_switches(mut self, switches: HashSet<String>) -> Self {
        self.switches = switches;
        self
    }

    pub fn settings(&self) -> &Map {
        &self.settings
    }

    pub fn with_settings(mut self, settings: Map) -> Self {
        if let Some(switches) = settings.get("switches") {
            if let Some(switches) = switches.clone().try_cast::<Vec<Dynamic>>() {
                self.switches = switches.into_iter().map(|v| v.to_string()).collect();
            }
        }
        self.settings = settings;
        self
    }

    pub fn defaults(&self) -> &HashSet<String> {
        &self.defaults
    }

    pub fn with_default<D: Into<String>>(mut self, default: D) -> Self {
        self.defaults.insert(default.into());
        self
    }
    
    pub fn with_defaults(mut self, defaults: HashSet<String>) -> Self {
        self.defaults = defaults;
        self
    }

    pub fn defaults_all(&self) -> bool {
        self.defaults_all
    }

    pub fn with_defaults_all(mut self, value: bool) -> Self {
        self.defaults_all = value;
        self
    }
}

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
