use std::borrow::Cow;

pub struct EntityModel {

}

pub struct Class<'i> {
    pub name: Cow<'i, str>,
    pub fields: Vec<Field<'i>>,
    pub methods: Vec<Method<'i>>,
}

impl<'i> Class<'i> {
    pub fn new<N: Into<Cow<'i, str>>>(name: N) -> Self {
        Class {
            name: name.into(),
            fields: vec![],
            methods: vec![]
        }
    }

    pub fn name(&self) -> &Cow<'i, str> {
        &self.name
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields[..]
    }

    pub fn methods(&self) -> &[Method] {
        &self.methods[..]
    }
}

pub struct Method<'i> {
    pub name: Cow<'i, str>,
    pub arguments: Vec<Argument<'i>>,
    pub returns: Type<'i>,
    pub access: Access,
}

pub struct Argument<'i> {
    pub name: Cow<'i, str>,
    pub typ: Type<'i>,
}

pub struct Field<'i> {
    pub name: Cow<'i, str>,
    pub typ: Type<'i>,
    pub access: Access,
}

pub enum Access {
    Default,
    Public,
    Private,
    Internal,
}

impl Default for Access {
    fn default() -> Self {
        Access::Default
    }
}

pub enum Type<'i> {
    Void,
    String,
    Char,
    I64,
    I32,
    U64,
    U32,
    I16,
    U16,
    I8,
    U8,
    F64,
    F32,
    Bool,
    List(Box<Type<'i>>),
    Map(Box<Type<'i>>, Box<Type<'i>>),
    Object(Cow<'i, str>),
}
