pub trait PromptInfo {
    fn message(&self) -> &str;
    fn key(&self) -> Option<&str>;

    fn optional(&self) -> bool;
    fn set_optional(&mut self, optional: bool);

    fn help(&self) -> Option<&str>;
    fn set_help(&mut self, value: Option<String>);

    fn placeholder(&self) -> Option<&str>;
    fn set_placeholder(&mut self, value: Option<String>);
}

pub trait PromptInfoLengthRestrictions: PromptInfo {
    fn min(&self) -> Option<i64>;
    fn set_min(&mut self, min: Option<i64>);

    fn max(&self) -> Option<i64>;
    fn set_max(&mut self, max: Option<i64>);
}

pub trait PromptInfoItemsRestrictions: PromptInfo {
    fn min_items(&self) -> Option<usize>;
    fn set_min_items(&mut self, min: Option<usize>);

    fn max_items(&self) -> Option<usize>;
    fn set_max_items(&mut self, max: Option<usize>);
}

pub trait PromptInfoPageable: PromptInfo {
    fn page_size(&self) -> Option<usize>;
    fn set_page_size(&mut self, min: Option<usize>);
}
