#[derive(Debug, Clone)]
pub enum Object{
    String(String),
    Number(f64),
    Boolean(bool),
    Nil
}

impl ToString for Object {
    fn to_string(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Boolean(b) => b.to_string(),
            Self::Nil => "nil".into(),
            Self::Number(n) => n.to_string(),
            _ => "".into()
        }
    }
}