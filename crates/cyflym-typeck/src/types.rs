#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Bool,
    String,
    Fn { params: Vec<Type>, ret: Box<Type> },
    Struct { name: String, fields: Vec<(String, Type)> },
    Enum { name: String, variants: Vec<(String, Vec<Type>)> },
    JoinHandle,
    Sender { inner: Box<Type> },
    Receiver { inner: Box<Type> },
    Mutex { inner: Box<Type> },
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Struct { name, .. } => write!(f, "{}", name),
            Type::Enum { name, .. } => write!(f, "{}", name),
            Type::JoinHandle => write!(f, "JoinHandle"),
            Type::Sender { inner } => write!(f, "Sender<{}>", inner),
            Type::Receiver { inner } => write!(f, "Receiver<{}>", inner),
            Type::Mutex { inner } => write!(f, "Mutex<{}>", inner),
            Type::Fn { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") {}", ret)
            }
        }
    }
}
