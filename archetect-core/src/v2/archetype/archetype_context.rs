use std::rc::Rc;
use camino::{Utf8Path, Utf8PathBuf};
use rhai::{Map};

#[derive(Clone)]
pub struct ArchetypeContext {
    inner: Rc<Inner>,
}

struct Inner {
    destination: Utf8PathBuf,
    answers: Map,
}

impl ArchetypeContext {
    pub fn new<T: Into<Utf8PathBuf>>(destination: T, answers: &Map) -> ArchetypeContext {
        ArchetypeContext {
            inner: Rc::new(Inner {
                destination: destination.into(),
                answers: create_owned_map(answers),
            })
        }
    }

    pub fn answers(&self) -> &Map {
        &self.inner.answers
    }

    pub fn destination(&self) -> &Utf8Path {
        self.inner.destination.as_path()
    }
}

fn create_owned_map(input: &Map) -> Map {
    let mut results = Map::new();
    results.extend(input.clone());
    results
}

#[cfg(test)]
mod tests {
    use rhai::{Dynamic, Engine, Map};
    use rhai::plugin::RhaiResult;
    use serde_json::Value;

    #[test]
    fn test_from_rhai() {
        let map = r#"
#{
    first_name: "Jimmie",
    last_name: "Fulton",
}
        "#;

        let list = r#"
["Jimmie", "Shirley", "Bailey"]
         "#;

        let string = r#"
            "Jimmie"
        "#;

        let int = r#"
            24
        "#;

        let engine = Engine::new();
        let result: Dynamic = engine.eval::<Dynamic>(int).unwrap();
        println!("{:?}", result.type_name());
    }

    #[test]
    fn test_from_json() {
        let map = r#"
{
    "first_name": "Jimmie",
    "last_name": "Fulton"
}
        "#;

        let list = r#"
["Jimmie", "Shirley", "Bailey"]
         "#;

        let string = r#"
            "Jimmie"
        "#;

        let int = r#"
            24
        "#;

        let result: Dynamic = serde_json::from_str(map).unwrap();
        // match result {
        //     Value::Null => println!("Null"),
        //     Value::Bool(value) => println!("Bool: {value:?}"),
        //     Value::Number(value) =>  println!("Number: {value:?}"),
        //     Value::String(value) =>  println!("String: {value:?}"),
        //     Value::Array(value) =>  println!("Array: {value:?}"),
        //     Value::Object(value) =>  println!("Object: {value:?}"),
        // }
        println!("{}", result.type_name());
        println!("{}", serde_json::to_string(&result).unwrap());

    }



}