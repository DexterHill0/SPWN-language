use crate::value::{Value, Error};

use crate::builtin::types::*;
use crate::builtin::types::classes::Instance;

pub trait ToValue {
    fn to_value(self) -> Value;
}

pub trait ToValueResult {
    fn to_value_result(self) -> Result<Value, Error>;
}

impl<R: ToValue> ToValueResult for R {
    fn to_value_result(self) -> Result<Value, Error> {
        Ok(self.to_value())
    }
}

impl ToValue for bool {
    fn to_value(self) -> Value {
        Value::Bool(self)
    }
}

impl ToValue for Group {
    fn to_value(self) -> Value {
        Value::Group(self)
    }
}

impl ToValue for TypeIndicator {
    fn to_value(self) -> Value {
        Value::TypeIndicator(self)
    }
}

impl ToValue for Instance {
    fn to_value(self) -> Value {
        Value::Instance(self)
    }
}

macro_rules! num_to_value {
    ($($n:ty)*) => {
        $(
            impl ToValue for $n {
                fn to_value(self) -> Value {
                    Value::Number(self as f64)
                }
            }
        )*
    };
}
num_to_value! { u8 u16 u32 u64 i16 i32 i64 f32 f64 }