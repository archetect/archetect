use rhai::EvalAltResult;
use inquire::InquireError;
use crate::ArchetypeError;

pub fn prompt(message: &str) -> Result<String, Box<EvalAltResult>> {
    let prompt = inquire::Editor::new(message);

    let result = prompt.prompt();
    match result {
        Ok(text) => Ok(text),
        Err(err) => match err {
            InquireError::OperationCanceled => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::ValueRequired),
                )));
            }
            InquireError::OperationInterrupted => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::OperationInterrupted),
                )));
            }
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
    }
}
