use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{
    error::RuntimeError,
    interpreter::{Globals, StoredValue, ValueKey},
};

use crate::{leveldata::object_data::GdObj, sources::CodeArea};

pub type ArbitraryId = u16;
pub type SpecificId = u16;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub enum Id {
    Specific(SpecificId),
    Arbitrary(ArbitraryId),
}
impl Id {
    pub fn to_str(&self) -> String {
        match self {
            Id::Specific(n) => n.to_string(),
            Id::Arbitrary(_) => "?".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),

    String(String),

    Bool(bool),

    Empty,

    Array(Vec<ValueKey>),
    Dict(HashMap<String, ValueKey>),
    Maybe(Option<ValueKey>),

    TypeIndicator(ValueType),
    Pattern(Pattern),

    Group(Id),
    TriggerFunc { start_group: Id },

    Macro(Macro),
    Object(GdObj),
}

#[derive(Debug, Clone)]
pub enum ValueIter {
    Array(Vec<ValueKey>, usize),
    Dict {
        dict_area: CodeArea,
        for_area: CodeArea,
        idx: usize,
        elems: Vec<(String, ValueKey)>,
    },
    String(String, CodeArea, usize),
}

impl ValueIter {
    pub fn next(&mut self, globals: &mut Globals) -> Option<StoredValue> {
        match self {
            ValueIter::Array(v, idx) => {
                *idx += 1;
                v.get(*idx - 1).map(|k| globals.deep_clone(*k))
            }
            ValueIter::Dict {
                dict_area,
                for_area,
                idx,
                elems,
            } => {
                *idx += 1;
                elems.get(*idx - 1).map(|(k, v)| {
                    Value::Array(vec![
                        globals
                            .memory
                            .insert(Value::String(k.clone()).into_stored(dict_area.clone())),
                        {
                            let val = globals.deep_clone(*v);
                            globals.memory.insert(val)
                        },
                    ])
                    .into_stored(for_area.clone())
                })
            }
            ValueIter::String(s, area, idx) => {
                *idx += 1;
                s.chars()
                    .nth(*idx - 1)
                    .map(|s| Value::String(s.to_string()).into_stored(area.clone()))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MacroArg {
    pub name: String,
    pub area: CodeArea,
    pub pattern: Option<(Pattern, CodeArea)>,
    pub default: Option<ValueKey>,
}
impl MacroArg {
    pub fn get_pattern(&self) -> Pattern {
        self.pattern.clone().map_or(Pattern::Any, |p| p.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Macro {
    pub func_id: usize,
    pub args: Vec<MacroArg>,
    pub capture: Vec<ValueKey>,
    pub ret_type: (Pattern, CodeArea),
}

impl Value {
    pub fn into_stored(self, area: CodeArea) -> StoredValue {
        StoredValue {
            value: self,
            def_area: area,
        }
    }
    pub fn get_type(&self) -> ValueType {
        match self {
            Value::Int(_) => ValueType::Int,
            Value::Float(_) => ValueType::Float,
            Value::String(_) => ValueType::String,
            Value::Bool(_) => ValueType::Bool,
            Value::Empty => ValueType::Empty,
            Value::Array(_) => ValueType::Array,
            Value::Dict(_) => ValueType::Dict,
            Value::Maybe(_) => ValueType::Maybe,
            Value::TypeIndicator(_) => ValueType::TypeIndicator,
            Value::Pattern(_) => ValueType::Pattern,
            Value::Group(_) => ValueType::Group,
            Value::TriggerFunc { .. } => ValueType::TriggerFunc,
            Value::Macro(_) => ValueType::Macro,
            Value::Object(_) => ValueType::Object,
        }
    }
    pub fn to_str(&self, globals: &Globals) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::String(v) => v.to_string(),
            Value::Bool(v) => v.to_string(),
            Value::Empty => "()".into(),
            Value::Array(arr) => format!(
                "[{}]",
                arr.iter()
                    .map(|v| globals.memory[*v].value.to_str(globals))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Dict(map) => format!(
                "{{{}}}",
                map.iter()
                    .map(|(k, v)| format!("{}: {}", k, globals.memory[*v].value.to_str(globals)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Maybe(None) => "?".into(),
            Value::Maybe(Some(v)) => format!("{}?", globals.memory[*v].value.to_str(globals)),
            Value::TypeIndicator(typ) => typ.to_str(),
            Value::Pattern(p) => p.to_str(),
            Value::Group(id) => format!("{}g", id.to_str()),
            Value::TriggerFunc { .. } => "!{...}".into(),
            Value::Macro(Macro { args, ret_type, .. }) => {
                format!(
                    "({}) -> {} {{...}}",
                    args.iter()
                        .map(
                            |MacroArg {
                                 name: n,
                                 pattern: t,
                                 default: d,
                                 ..
                             }| {
                                format!(
                                    "{}{}{}",
                                    n,
                                    if let Some((t, _)) = t {
                                        format!(": {}", t.to_str())
                                    } else {
                                        "".into()
                                    },
                                    if let Some(d) = d {
                                        format!(" = {}", globals.memory[*d].value.to_str(globals))
                                    } else {
                                        "".into()
                                    },
                                )
                            }
                        )
                        .collect::<Vec<_>>()
                        .join(", "),
                    ret_type.0.to_str(),
                )
            }
            Value::Object(a) => format!("{:?}", a),
        }
    }
    pub fn deep_clone(&self, globals: &mut Globals) -> Value {
        match self {
            Value::Int(_)
            | Value::Float(_)
            | Value::String(_)
            | Value::Bool(_)
            | Value::Empty
            | Value::TypeIndicator(_)
            | Value::Pattern(_)
            | Value::Group(_)
            | Value::TriggerFunc { .. }
            | Value::Object(_) => self.clone(),
            Value::Array(arr) => Value::Array(
                arr.iter()
                    .map(|v| globals.key_deep_clone(*v))
                    .collect::<Vec<_>>(),
            ),
            Value::Dict(map) => Value::Dict(
                map.iter()
                    .map(|(k, v)| (k.clone(), globals.key_deep_clone(*v)))
                    .collect(),
            ),
            Value::Maybe(v) => Value::Maybe(v.map(|v| globals.key_deep_clone(v))),
            Value::Macro(Macro {
                func_id,
                args,
                ret_type,
                capture,
            }) => {
                let args = args
                    .iter()
                    .map(|m| MacroArg {
                        name: m.name.clone(),
                        area: m.area.clone(),
                        pattern: m.pattern.clone(),
                        default: m.default.map(|d| globals.key_deep_clone(d)),
                    })
                    .collect();
                Value::Macro(Macro {
                    func_id: *func_id,
                    args,
                    ret_type: ret_type.clone(),
                    capture: capture.clone(),
                })
            }
        }
    }
}

impl StoredValue {
    pub fn deep_clone(&self, globals: &mut Globals) -> StoredValue {
        StoredValue {
            value: self.value.deep_clone(globals),
            def_area: self.def_area.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Copy)]
pub enum ValueType {
    Int,
    Float,
    String,
    Bool,
    Empty,
    Array,
    Dict,
    Maybe,
    TypeIndicator,
    Pattern,
    Group,
    TriggerFunc,
    Macro,
    Object,
    // more soon
}
impl ValueType {
    pub fn to_str(&self) -> String {
        format!(
            "@{}",
            match self {
                ValueType::Int => "int",
                ValueType::Float => "float",
                ValueType::String => "string",
                ValueType::Bool => "bool",
                ValueType::Empty => "empty",
                ValueType::Array => "array",
                ValueType::Dict => "dictionary",
                ValueType::Maybe => "maybe",
                ValueType::TypeIndicator => "type_indicator",
                ValueType::Pattern => "pattern",
                ValueType::Group => "group",
                ValueType::TriggerFunc => "trigger_function",
                ValueType::Macro => "macro",
                ValueType::Object => "object",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Pattern {
    Any,
    Type(ValueType),
    Macro {
        args: Vec<Pattern>,
        ret: Box<Pattern>,
    },
}
impl Pattern {
    pub fn to_str(&self) -> String {
        match self {
            Pattern::Any => "_".into(),
            Pattern::Type(t) => t.to_str(),
            Pattern::Macro { args, ret } => format!(
                "({}) -> {}",
                args.iter()
                    .map(|arg| arg.to_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                ret.to_str(),
            ),
        }
    }
}

// ok so this is a temporary thing until we get builtins and i can replace this with _plus_ and such
pub mod value_ops {
    use super::super::error::RuntimeError;
    use super::super::interpreter::StoredValue;
    use super::Macro;
    use super::Pattern;
    use super::Value;
    use super::ValueIter;
    use super::ValueType;

    use crate::interpreter::interpreter::Globals;
    use crate::sources::CodeArea;

    pub fn equality(a: &Value, b: &Value, globals: &Globals) -> bool {
        match (a, b) {
            (Value::Int(n1), Value::Float(n2)) => *n1 as f64 == *n2,
            (Value::Float(n1), Value::Int(n2)) => *n1 == *n2 as f64,

            (Value::Array(arr1), Value::Array(arr2)) => {
                if arr1.len() != arr2.len() {
                    false
                } else {
                    arr1.iter().zip(arr2).all(|(a, b)| {
                        equality(
                            &globals.memory[*a].value,
                            &globals.memory[*b].value,
                            globals,
                        )
                    })
                }
            }
            (Value::Dict(map1), Value::Dict(map2)) => {
                if map1.len() != map2.len() {
                    false
                } else {
                    for (k, a) in map1 {
                        match map2.get(k) {
                            Some(b) => {
                                if !equality(
                                    &globals.memory[*a].value,
                                    &globals.memory[*b].value,
                                    globals,
                                ) {
                                    return false;
                                }
                            }
                            None => return false,
                        }
                    }
                    true
                }
            }

            (Value::Maybe(None), Value::Maybe(None)) => true,
            (Value::Maybe(Some(a)), Value::Maybe(Some(b))) => equality(
                &globals.memory[*a].value,
                &globals.memory[*b].value,
                globals,
            ),

            _ => a == b,
        }
    }

    pub fn matches_pat(val: &Value, pat: &Pattern) -> bool {
        match (val, pat) {
            (_, Pattern::Any) => true,
            (_, Pattern::Type(t)) => &val.get_type() == t,
            (
                Value::Macro(Macro {
                    func_id,
                    args,
                    capture,
                    ret_type,
                }),
                Pattern::Macro {
                    args: arg_patterns,
                    ret: ret_pattern,
                },
            ) => {
                &ret_type.0 == &**ret_pattern
                    && args
                        .iter()
                        .zip(arg_patterns)
                        .all(|(a, p)| &a.get_pattern() == p)
            }
            (_, _) => false,
        }
    }

    pub fn to_bool(a: &StoredValue) -> Result<bool, RuntimeError> {
        match &a.value {
            Value::Bool(b) => Ok(*b),
            _ => Err(RuntimeError::CannotConvert {
                a: a.clone(),
                to: ValueType::Bool,
            }),
        }
    }

    pub fn to_pat(a: &StoredValue) -> Result<Pattern, RuntimeError> {
        match &a.value {
            Value::TypeIndicator(typ) => Ok(Pattern::Type(*typ)),
            Value::Pattern(p) => Ok(p.clone()),
            _ => Err(RuntimeError::CannotConvert {
                a: a.clone(),
                to: ValueType::Pattern,
            }),
        }
    }

    pub fn to_iter(a: &StoredValue, for_area: CodeArea) -> Result<ValueIter, RuntimeError> {
        match &a.value {
            Value::Array(v) => Ok(ValueIter::Array(v.clone(), 0)),
            Value::String(s) => Ok(ValueIter::String(s.clone(), a.def_area.clone(), 0)),
            Value::Dict(map) => Ok(ValueIter::Dict {
                dict_area: a.def_area.clone(),
                for_area,
                idx: 0,
                elems: map.iter().map(|(k, v)| (k.clone(), *v)).collect::<Vec<_>>(),
            }),
            _ => Err(RuntimeError::CannotIterate { a: a.clone() }),
        }
    }

    pub fn plus(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => Value::Int(*n1 + *n2),
            (Value::Int(n1), Value::Float(n2)) => Value::Float(*n1 as f64 + *n2),
            (Value::Float(n1), Value::Int(n2)) => Value::Float(*n1 + *n2 as f64),
            (Value::Float(n1), Value::Float(n2)) => Value::Float(*n1 + *n2),
            (Value::String(s1), Value::String(s2)) => Value::String(s1.clone() + s2),

            (Value::Array(arr1), Value::Array(arr2)) => {
                Value::Array(arr1.iter().chain(arr2).cloned().collect::<Vec<_>>())
            }

            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: "+".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn minus(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => Value::Int(*n1 - *n2),
            (Value::Int(n1), Value::Float(n2)) => Value::Float(*n1 as f64 - *n2),
            (Value::Float(n1), Value::Int(n2)) => Value::Float(*n1 - *n2 as f64),
            (Value::Float(n1), Value::Float(n2)) => Value::Float(*n1 - *n2),
            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: "-".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn mult(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => Value::Int(*n1 * *n2),
            (Value::Int(n1), Value::Float(n2)) => Value::Float(*n1 as f64 * *n2),
            (Value::Float(n1), Value::Int(n2)) => Value::Float(*n1 * *n2 as f64),
            (Value::Float(n1), Value::Float(n2)) => Value::Float(*n1 * *n2),

            (Value::Int(n), Value::String(s)) => {
                Value::String(s.repeat(if *n < 0 { 0 } else { *n as usize }))
            }
            (Value::String(s), Value::Int(n)) => {
                Value::String(s.repeat(if *n < 0 { 0 } else { *n as usize }))
            }
            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: "*".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn div(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => Value::Int(*n1 / *n2),
            (Value::Int(n1), Value::Float(n2)) => Value::Float(*n1 as f64 / *n2),
            (Value::Float(n1), Value::Int(n2)) => Value::Float(*n1 / *n2 as f64),
            (Value::Float(n1), Value::Float(n2)) => Value::Float(*n1 / *n2),
            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: "/".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn modulo(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => Value::Int(*n1 % *n2),
            (Value::Int(n1), Value::Float(n2)) => Value::Float(*n1 as f64 % *n2),
            (Value::Float(n1), Value::Int(n2)) => Value::Float(*n1 % *n2 as f64),
            (Value::Float(n1), Value::Float(n2)) => Value::Float(*n1 % *n2),
            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: "%".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn pow(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => {
                Value::Int((*n1 as f64).powf(*n2 as f64).floor() as i64)
            }
            (Value::Int(n1), Value::Float(n2)) => Value::Float((*n1 as f64).powf(*n2)),
            (Value::Float(n1), Value::Int(n2)) => Value::Float((*n1).powf(*n2 as f64)),
            (Value::Float(n1), Value::Float(n2)) => Value::Float(n1.powf(*n2)),
            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: "^".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn eq(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        Ok(Value::Bool(equality(&a.value, &b.value, globals)).into_stored(area))
    }
    pub fn not_eq(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        Ok(Value::Bool(!equality(&a.value, &b.value, globals)).into_stored(area))
    }
    pub fn greater(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => Value::Bool(*n1 > *n2),
            (Value::Int(n1), Value::Float(n2)) => Value::Bool(*n1 as f64 > *n2),
            (Value::Float(n1), Value::Int(n2)) => Value::Bool(*n1 > *n2 as f64),
            (Value::Float(n1), Value::Float(n2)) => Value::Bool(*n1 > *n2),
            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: ">".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn greater_eq(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => Value::Bool(*n1 >= *n2),
            (Value::Int(n1), Value::Float(n2)) => Value::Bool(*n1 as f64 >= *n2),
            (Value::Float(n1), Value::Int(n2)) => Value::Bool(*n1 >= *n2 as f64),
            (Value::Float(n1), Value::Float(n2)) => Value::Bool(*n1 >= *n2),
            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: ">=".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn lesser(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => Value::Bool(*n1 < *n2),
            (Value::Int(n1), Value::Float(n2)) => Value::Bool((*n1 as f64) < *n2),
            (Value::Float(n1), Value::Int(n2)) => Value::Bool(*n1 < *n2 as f64),
            (Value::Float(n1), Value::Float(n2)) => Value::Bool(*n1 < *n2),
            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: "<".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn lesser_eq(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (Value::Int(n1), Value::Int(n2)) => Value::Bool(*n1 <= *n2),
            (Value::Int(n1), Value::Float(n2)) => Value::Bool(*n1 as f64 <= *n2),
            (Value::Float(n1), Value::Int(n2)) => Value::Bool(*n1 <= *n2 as f64),
            (Value::Float(n1), Value::Float(n2)) => Value::Bool(*n1 <= *n2),
            _ => {
                return Err(RuntimeError::InvalidOperands {
                    a: a.clone(),
                    b: b.clone(),
                    op: "<=".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }

    pub fn unary_negate(a: &StoredValue, area: CodeArea) -> Result<StoredValue, RuntimeError> {
        let value = match &a.value {
            Value::Int(n) => Value::Int(-n),
            Value::Float(n) => Value::Float(-n),
            _ => {
                return Err(RuntimeError::InvalidUnaryOperand {
                    a: a.clone(),
                    op: "-".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn unary_not(a: &StoredValue, area: CodeArea) -> Result<StoredValue, RuntimeError> {
        let value = match &a.value {
            Value::Bool(n) => Value::Bool(!n),
            _ => {
                return Err(RuntimeError::InvalidUnaryOperand {
                    a: a.clone(),
                    op: "-".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
    pub fn is_op(
        a: &StoredValue,
        b: &StoredValue,
        area: CodeArea,
        globals: &Globals,
    ) -> Result<StoredValue, RuntimeError> {
        let value = match (&a.value, &b.value) {
            (a, Value::TypeIndicator(typ)) => Value::Bool(&a.get_type() == typ),
            (a, Value::Pattern(pat)) => Value::Bool(matches_pat(a, pat)),
            (_, _) => {
                return Err(RuntimeError::TypeMismatch {
                    v: b.clone(),
                    expected: "@type_indicator or @pattern".into(),
                    area,
                })
            }
        };
        Ok(value.into_stored(area))
    }
}
