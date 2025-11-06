use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataModel<'a> {
    pub name: Option<Cow<'a, str>>,
    pub fields: Vec<DataModelField<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataModelField<'a> {
    Object {
        name: Option<Cow<'a, str>>,
        description: Option<Cow<'a, str>>,
        fields: Vec<DataModelField<'a>>,
    },
    Array {
        name: Option<Cow<'a, str>>,
        description: Option<Cow<'a, str>>,
        items: Box<DataModelField<'a>>,
    },
    String {
        name: Option<Cow<'a, str>>,
        description: Option<Cow<'a, str>>,
    },
    Integer {
        name: Option<Cow<'a, str>>,
        description: Option<Cow<'a, str>>,
    },
    Float {
        name: Option<Cow<'a, str>>,
        description: Option<Cow<'a, str>>,
    },
    Boolean {
        name: Option<Cow<'a, str>>,
        description: Option<Cow<'a, str>>,
    },
}
