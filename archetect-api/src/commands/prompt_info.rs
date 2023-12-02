pub trait PromptInfo {
    fn message(&self) -> &str;
    fn optional(&self) -> bool;

    fn help(&self) -> Option<&str>;

    fn placeholder(&self) -> Option<&str>;
}
