use std::collections::HashMap;

use ahash::{AHashMap, AHashSet};
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

use super::contexts::{Context, FullContext, SkipMode};
use super::error::RuntimeError;
use super::value::{value_ops, ArbitraryId, Value};

use crate::compiler::compiler::{Code, Instruction};
use crate::interpreter::contexts::{Block, Frame, ReturnType};
use crate::interpreter::value::{Id, Macro, MacroArg, Pattern};
use crate::leveldata::object_data::{GdObj, ObjParam, ObjectMode};
use crate::parser::ast::IdClass;
use crate::sources::CodeArea;

new_key_type! {
    pub struct ValueKey;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StoredValue {
    pub value: Value,
    pub def_area: CodeArea,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CallId(pub usize);

type InstructionPos = (usize, usize);

pub struct Globals
where
    Self: Send + Sync,
{
    pub memory: SlotMap<ValueKey, StoredValue>,

    // pub types: AHashMap<String, Type>,
    // pub instances: AHashMap<u32, Instance>,
    pub instance_id: u32,

    pub arbitrary_ids: [ArbitraryId; 4],

    pub objects: Vec<GdObj>,
    pub triggers: Vec<GdObj>,
    // merge_points: AHashMap<(InstructionPos, CallId), Vec<(Context, Id)>>,
}
impl Globals {
    pub fn new() -> Self {
        Self {
            memory: SlotMap::default(),
            // types: AHashMap::new(),
            // instances: AHashMap::new(),
            instance_id: 0,
            arbitrary_ids: [0; 4],
            objects: vec![],
            triggers: vec![],
        }
    }
    pub fn init(&mut self) {
        // self.types.insert("int".into(), ValueType::Int);
        // self.types.insert("float".into(), ValueType::Float);
        // self.types.insert("string".into(), ValueType::String);
        // self.types.insert("bool".into(), ValueType::Bool);
        // self.types.insert("empty".into(), ValueType::Empty);
        // self.types.insert("array".into(), ValueType::Array);
        // self.types.insert("dictionary".into(), ValueType::Dict);
        // self.types.insert("maybe".into(), ValueType::Maybe);
        // self.types
        //     .insert("type_indicator".into(), ValueType::TypeIndicator);
        // self.types.insert("pattern".into(), ValueType::Pattern);
        // self.types.insert("group".into(), ValueType::Group);
        // self.types
        //     .insert("trigger_function".into(), ValueType::TriggerFunc);
        // self.types.insert("macro".into(), ValueType::Macro);
    }
    pub fn key_deep_clone(&mut self, k: ValueKey) -> ValueKey {
        let val = self.memory[k].clone();
        let val = val.deep_clone(self);
        self.memory.insert(val)
    }
    pub fn deep_clone(&mut self, k: ValueKey) -> StoredValue {
        let val = self.memory[k].clone();
        val.deep_clone(self)
    }
}

pub fn execute_code(
    globals: &mut Globals,
    code: &Code,
    func: usize,
    contexts: &mut FullContext,
    args: Vec<StoredValue>,
    captured: Vec<ValueKey>,
) -> Result<(), RuntimeError> {
    for context in contexts.iter(SkipMode::IncludeReturns) {
        assert!(context.inner().returned.is_none());
        // todo: clone vars and shit to clean up later

        for (arg, id) in args.iter().zip(&code.bytecode_funcs[func].arg_ids) {
            let k = globals.memory.insert(arg.clone()); // deep clone ??
            context.inner().set_var(*id, k);
        }
    }

    loop {
        let mut all_returned = true;
        'out_for: for context in contexts.iter(SkipMode::SkipReturns) {
            all_returned = false;
            macro_rules! pop_deep_clone {
                () => {{
                    let val = globals.memory[context.inner().stack.pop().unwrap()].clone();
                    val.deep_clone(globals)
                }};
                (Store) => {{
                    globals.key_deep_clone(context.inner().stack.pop().unwrap())
                }};
            }

            macro_rules! pop_ref {
                () => {
                    &globals.memory[context.inner().stack.pop().unwrap()]
                };
            }

            macro_rules! pop_shallow {
                () => {
                    globals.memory[context.inner().stack.pop().unwrap()].clone()
                };
            }

            macro_rules! push {
                ($v:expr) => {{
                    let key = globals.memory.insert($v);
                    context.inner().stack.push(key);
                }};
            }

            macro_rules! push_store {
                ($v:expr) => {{
                    #[allow(unused_unsafe)]
                    let key = globals.memory.insert($v);
                    context.inner().stack.push(key);
                }};
            }

            macro_rules! store {
                ($v:expr) => {
                    globals.memory.insert($v)
                };
            }
            macro_rules! try_err {
                ($err:expr) => {
                    loop {
                        match context.inner().block_stack.pop() {
                            Some(Block::Try(to)) => {
                                context.inner().jump_to(code, to, func);
                                //break 'out_for;
                            }
                            None => return Err($err),
                            _ => (),
                        }
                    }
                };
                (?: $e:expr) => {
                    match $e {
                        Ok(e) => e,
                        Err(err) => loop {
                            match context.inner().block_stack.pop() {
                                Some(Block::Try(to)) => {
                                    context.inner().jump_to(code, to, func);
                                    break 'out_for;
                                }
                                None => return Err(err),
                                _ => (),
                            }
                        },
                    }
                };
            }
            macro_rules! op_helper {
                (
                    $($instr:ident: $func:ident,)*
                ) => {
                    match &code.bytecode_funcs[func].instructions[context.inner().pos] {
                        $(
                            Instruction::$instr => {
                                let span = code.get_bytecode_span(func, context.inner().pos);
                                let b = pop_ref!();
                                let a = pop_ref!();
                                let key = globals.memory.insert(try_err!(?: value_ops::$func(a, b, code.make_area(span), globals)));
                                context.inner().stack.push(key);
                            }
                        )*
                        _ => (),
                    }
                };
            }

            op_helper! {
                Plus: plus,
                Minus: minus,
                Mult: mult,
                Div: div,
                Mod: modulo,
                Pow: pow,
                Eq: eq,
                NotEq: not_eq,
                Greater: greater,
                GreaterEq: greater_eq,
                Lesser: lesser,
                LesserEq: lesser_eq,
                Is: is_op,
            };

            match &code.bytecode_funcs[func].instructions[*(&mut context.inner().pos)] {
                Instruction::LoadConst(id) => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    let key = globals.memory.insert(
                        code.constants
                            .get(*id)
                            .clone()
                            .to_value()
                            .into_stored(code.make_area(span)),
                    );
                    context.inner().stack.push(key);
                }
                Instruction::Negate => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));

                    push_store!(
                        try_err!(?: value_ops::unary_negate(pop_ref!(), code.make_area(span)))
                    );
                }
                Instruction::Not => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));

                    push_store!(
                        try_err!(?: value_ops::unary_not(pop_ref!(), code.make_area(span)))
                    );
                }
                Instruction::LoadVar(id) => {
                    let v = context.inner().get_var(*id);
                    context.inner().stack.push(v)
                }
                Instruction::SetVar(id) => {
                    let v = pop_deep_clone!();
                    context.inner().modify_var(*id, v, globals)
                }
                Instruction::Print => {
                    let top = pop_ref!();
                    println!(
                        "{}, ctx: {}",
                        ansi_term::Color::Green
                            .bold()
                            .paint(top.value.to_str(globals)),
                        ansi_term::Color::Blue
                            .bold()
                            .paint(format!("{:?}", context.inner().group))
                    );
                    // for (i, vars) in context.inner().vars.iter().enumerate() {
                    //     println!(
                    //         "{} [{}]",
                    //         i,
                    //         vars.iter()
                    //             .map(|k| globals.memory[*k].value.to_str(globals))
                    //             .collect::<Vec<_>>()
                    //             .join(", ")
                    //     )
                    // }
                }
                Instruction::LoadType(id) => {
                    todo!()
                    // let span = code.get_bytecode_span(func, i);
                    // let name = code.names.get(*id);
                    // match globals.types.get(name) {
                    //     Some(typ) => {
                    //         push!(Value::TypeIndicator(*typ).into_stored(code.make_area(span)))
                    //     }
                    //     None => {
                    //         try_err!(RuntimeError::UndefinedType {
                    //             name: name.clone(),
                    //             area: code.make_area(span),
                    //         })
                    //     }
                    // }
                }
                Instruction::BuildArray(len) => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    let mut elems = vec![];
                    for _ in 0..*len {
                        elems.push(pop_deep_clone!(Store));
                    }
                    elems.reverse();
                    push!(Value::Array(elems).into_stored(code.make_area(span)));
                }
                Instruction::PushEmpty => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    push!(Value::Empty.into_stored(code.make_area(span)));
                }
                Instruction::PopTop => {
                    context.inner().stack.pop();
                }
                Instruction::Jump(id) => {
                    *(&mut context.inner().pos) = *code.destinations.get(*id) - 1;
                }
                Instruction::JumpIfFalse(id) => {
                    if !try_err!(?: value_ops::to_bool(pop_ref!())) {
                        *(&mut context.inner().pos) = *code.destinations.get(*id) - 1;
                    }
                }
                Instruction::ToIter => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    let iter =
                        try_err!(?: value_ops::to_iter(&pop_shallow!(), code.make_area(span)));
                    context.inner().block_stack.push(Block::For(iter))
                }
                Instruction::IterNext(to) => {
                    if let Some(v) = context.inner().block().get_iter().next(globals) {
                        push_store!(v);
                    } else {
                        *(&mut context.inner().pos) = *code.destinations.get(*to) - 1;
                    }
                }
                Instruction::PopBlock(amount) => {
                    for _ in 0..*amount {
                        context.inner().block_stack.pop();
                    }
                }
                Instruction::BuildDict(id) => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    let keys = code.name_sets.get(*id);
                    let map = keys
                        .iter()
                        .cloned()
                        .zip((0..keys.len()).map(|_| pop_deep_clone!(Store)))
                        .collect();
                    push!(Value::Dict(map).into_stored(code.make_area(span)));
                }
                mode @ (Instruction::BuildObject(n) | Instruction::BuildTrigger(n)) => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    let mut obj = GdObj {
                        params: HashMap::new(),
                        mode: match mode {
                            Instruction::BuildObject(_) => ObjectMode::Object,
                            Instruction::BuildTrigger(_) => ObjectMode::Trigger,
                            _ => unreachable!(),
                        },
                    };
                    for _ in 0..*n {
                        let val = pop_deep_clone!();
                        let key = pop_shallow!();
                        // make sure key is number (for now)
                        let key = match key.value {
                            Value::Int(n) => n as u16,
                            _ => {
                                try_err!(RuntimeError::TypeMismatch {
                                    v: key.clone(),
                                    expected: "integer".to_string(),
                                    area: key.def_area,
                                })
                            }
                        };
                        // convert to obj param
                        let param = match val.value {
                            Value::Int(n) => ObjParam::Number(n as f64),
                            Value::Float(x) => ObjParam::Number(x),
                            Value::String(s) => ObjParam::Text(s),
                            Value::Bool(b) => ObjParam::Bool(b),
                            Value::Group(g) => ObjParam::Group(g),
                            Value::TriggerFunc { start_group } => ObjParam::Group(start_group),
                            _ => {
                                try_err!(RuntimeError::TypeMismatch {
                                    v: val.clone(),
                                    expected: "valid object property value".to_string(),
                                    area: val.def_area,
                                })
                            }
                        };
                        obj.params.insert(key, param);
                    }
                    push!(Value::Object(obj).into_stored(code.make_area(span)));
                }

                Instruction::AddObject => {
                    let object = pop_shallow!();
                    match object.value {
                        Value::Object(mut obj) => match obj.mode {
                            ObjectMode::Object => globals.objects.push(obj),
                            ObjectMode::Trigger => {
                                obj.params
                                    .insert(57, ObjParam::Group(context.inner().group));
                                globals.triggers.push(obj)
                            }
                        },
                        _ => {
                            try_err!(RuntimeError::TypeMismatch {
                                v: object.clone(),
                                expected: "obj or trigger".to_string(),
                                area: object.def_area,
                            })
                        }
                    };
                }
                Instruction::MakeMacro(id) => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    let arg_spans = code
                        .macro_arg_spans
                        .get(&(func, *(&mut context.inner().pos)))
                        .unwrap();
                    let (func_id, arg_info) = code.macro_build_info.get(*id);
                    let ret_type = {
                        let value = pop_shallow!();
                        (try_err!(?: value_ops::to_pat(&value)), value.def_area)
                    };
                    let mut args = vec![];
                    for ((name, typ, def), span) in arg_info.iter().zip(arg_spans) {
                        let def = if *def {
                            Some(pop_deep_clone!(Store))
                        } else {
                            None
                        };
                        let typ = if *typ {
                            let value = pop_shallow!();

                            Some((try_err!(?: value_ops::to_pat(&value)), value.def_area))
                        } else {
                            None
                        };
                        args.push(MacroArg {
                            name: name.clone(),
                            area: code.make_area(*span),
                            pattern: typ,
                            default: def,
                        });
                    }
                    args.reverse();
                    // println!("balls {:?}", )
                    let capture = code.bytecode_funcs[*func_id]
                        .capture_ids
                        .iter()
                        .map(|id| context.inner().get_var(*id))
                        .collect::<Vec<_>>();
                    push!(Value::Macro(Macro {
                        func_id: *func_id,
                        args,
                        ret_type,
                        capture,
                    })
                    .into_stored(code.make_area(span)));
                }
                Instruction::PushAnyPattern => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    push!(Value::Pattern(Pattern::Any).into_stored(code.make_area(span)));
                }
                Instruction::MakeMacroPattern(arg_amount) => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    let ret = try_err!(?: value_ops::to_pat(&pop_shallow!()));
                    let mut args = vec![];
                    for i in 0..*arg_amount {
                        args.push(try_err!(?: value_ops::to_pat(&pop_shallow!())));
                    }
                    args.reverse();
                    push!(Value::Pattern(Pattern::Macro {
                        args,
                        ret: Box::new(ret)
                    })
                    .into_stored(code.make_area(span)));
                }
                Instruction::Index => {
                    let idx = pop_shallow!();
                    let base = pop_shallow!();
                    match &base.value {
                        Value::Array(arr) => match idx.value {
                            Value::Int(n) => {
                                let k = globals.deep_clone(arr[n as usize]);
                                push!(k);
                            }
                            _ => {
                                try_err!(RuntimeError::TypeMismatch {
                                    v: idx.clone(),
                                    expected: "integer".to_string(),
                                    area: idx.def_area,
                                })
                            }
                        },
                        _ => {
                            try_err!(RuntimeError::TypeMismatch {
                                v: base.clone(),
                                expected: "array".to_string(),
                                area: base.def_area,
                            })
                        }
                    }
                }
                Instruction::Call(id) => {
                    todo!()
                    // let span = code.get_bytecode_span(func, *pos);
                    // let base = pop_shallow!();
                    // match &base.value {
                    //     Value::Macro(m) => {
                    //         if let Some(value) =
                    //             macro_call(code, func, *pos, id, m, &base, globals, context)
                    //         {
                    //             return value;
                    //         }

                    //         continue;
                    //     }
                    //     _ => {
                    //         try_err!(RuntimeError::CannotCall {
                    //             base: base.clone(),
                    //             area: code.make_area(span),
                    //         })
                    //     }
                    // }
                }
                Instruction::ReturnValue => {
                    context.inner().returned = Some(ReturnType::Explicit);
                    continue;
                }
                Instruction::TriggerFuncCall => {
                    let v = pop_shallow!();
                    let trigger_func = match v.value {
                        Value::TriggerFunc { start_group } => start_group,
                        _ => {
                            try_err!(RuntimeError::TypeMismatch {
                                v: v.clone(),
                                expected: "trigger function".to_string(),
                                area: v.def_area,
                            })
                        }
                    };
                    let mut obj = GdObj {
                        params: HashMap::new(),
                        mode: ObjectMode::Trigger,
                    };

                    obj.params.insert(1, ObjParam::Number(1268.0));
                    obj.params.insert(51, ObjParam::Group(trigger_func));
                    obj.params
                        .insert(57, ObjParam::Group(context.inner().group));
                    globals.triggers.push(obj);
                }
                Instruction::MergeContexts => {

                    // if context.is_split() {
                    //     let call_id = context.inner().call_id;
                    //     // check if can merge
                    //     if let Some(cs) = globals.merge_points.get(&((func, *pos), call_id)) {
                    //         for (c, group) in cs {
                    //             if context.inner().is_mergable_with(c, globals) {
                    //                 // add spawn trigger
                    //                 add_spawn_trigger(*group, context, globals);

                    //                 // yeet merged context
                    //                 context.inner().yeet();
                    //                 break 'out_for;
                    //             }
                    //         }
                    //     }

                    //     // otherwise
                    //     let new_context =
                    //         Id::Arbitrary(globals.arbitrary_ids[IdClass::Group as usize]);
                    //     globals.arbitrary_ids[IdClass::Group as usize] += 1;

                    //     // add spawn trigger
                    //     add_spawn_trigger(new_context, context, globals);

                    //     context.inner().set_group(new_context);

                    //     globals
                    //         .merge_points
                    //         .entry(((func, *pos), call_id))
                    //         .or_insert(vec![])
                    //         .push((context.inner().clone(), new_context));
                    // }
                }
                Instruction::PushNone => {
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    push!(Value::Maybe(None).into_stored(code.make_area(span)));
                }
                Instruction::WrapMaybe => {
                    let top = pop_deep_clone!();
                    let span = code.get_bytecode_span(func, *(&mut context.inner().pos));
                    let key = store!(top);
                    push!(Value::Maybe(Some(key)).into_stored(code.make_area(span)));
                }
                Instruction::EnterTriggerFunction(id) => {
                    // gets the area of the trigger function
                    let trig_fn_span = code.get_bytecode_span(func, context.inner().pos);
                    let trig_fn_group =
                        Id::Arbitrary(globals.arbitrary_ids[IdClass::Group as usize]);
                    globals.arbitrary_ids[IdClass::Group as usize] += 1;

                    context.split_context(globals);
                    let current_pos = context.inner().pos;
                    match context {
                        FullContext::Single(_) => unreachable!(),
                        FullContext::Split(outside, inside) => {
                            outside
                                .inner()
                                .jump_to(code, *code.destinations.get(*id), func);
                            inside.inner().set_group(trig_fn_group);

                            outside.inner().stack.push(store!(Value::TriggerFunc {
                                start_group: trig_fn_group
                            }
                            .into_stored(code.make_area(trig_fn_span))));
                            inside.inner().jump_to(code, current_pos + 1, func);

                            break 'out_for;
                        }
                    }
                }

                Instruction::EnterArrowStatement(id) => {
                    context.split_context(globals);
                    let current_pos = context.inner().pos;
                    match context {
                        FullContext::Single(_) => unreachable!(),
                        FullContext::Split(outside, inside) => {
                            outside
                                .inner()
                                .jump_to(code, *code.destinations.get(*id), func);
                            inside.inner().jump_to(code, current_pos + 1, func);
                            break 'out_for;
                        }
                    }
                }
                // ?g
                Instruction::LoadArbitraryId(class) => {
                    let span = code.get_bytecode_span(func, context.inner().pos);
                    let id = Id::Arbitrary(globals.arbitrary_ids[*class as usize]);
                    globals.arbitrary_ids[*class as usize] += 1;
                    push!(match class {
                        IdClass::Group => Value::Group(id),
                        IdClass::Color => Value::Color(id),
                        IdClass::Block => Value::Block(id),
                        IdClass::Item => Value::Item(id),
                    }
                    .into_stored(code.make_area(span)));
                }
                Instruction::YeetContext => {
                    context.inner().yeet();
                    continue;
                }

                Instruction::TypeDef(_) => todo!(),
                Instruction::Impl(_) => todo!(),
                Instruction::Instance(_) => todo!(),
                Instruction::Split => {
                    let b = pop_deep_clone!(Store);
                    let a = pop_deep_clone!(Store);
                    context.split_context(globals);
                    let current_pos = context.inner().pos;
                    match context {
                        FullContext::Single(_) => unreachable!(),
                        FullContext::Split(c_a, c_b) => {
                            c_a.inner().stack.push(a);
                            c_b.inner().stack.push(b);
                            c_a.inner().jump_to(code, current_pos + 1, func);
                            c_b.inner().jump_to(code, current_pos + 1, func);
                            break 'out_for;
                        }
                    }
                }

                // Instruction::PushVars(idx) => {
                //     context
                //         .inner()
                //         .push_vars(code.scope_vars.get(*idx), code, globals);
                // }
                // Instruction::PopVars(idx) => {
                //     context.inner().pop_vars(code.scope_vars.get(*idx));
                // }
                Instruction::Plus
                | Instruction::Minus
                | Instruction::Mult
                | Instruction::Div
                | Instruction::Mod
                | Instruction::Pow
                | Instruction::Eq
                | Instruction::NotEq
                | Instruction::Greater
                | Instruction::GreaterEq
                | Instruction::Lesser
                | Instruction::LesserEq
                | Instruction::Is => (),

                Instruction::PushTry(to) => {
                    let to = *code.destinations.get(*to);
                    context.inner().block_stack.push(Block::Try(to));
                }
            }

            let new_pos = context.inner().pos + 1;
            context.inner().jump_to(code, new_pos, func);
        }

        contexts.clean_yeeted();
        if all_returned {
            break;
        }
    }

    let explicitly_returned = contexts
        .iter(SkipMode::IncludeReturns)
        .any(|c| matches!(c.inner().returned, Some(ReturnType::Explicit)));

    if explicitly_returned {
        contexts.yeet_implicit();
        contexts.clean_yeeted();
    }

    // reset return states
    for context in contexts.iter(SkipMode::IncludeReturns) {
        context.inner().returned = None;
    }

    Ok(())
}

fn add_spawn_trigger(target: Id, context: &mut FullContext, globals: &mut Globals) {
    let mut obj = GdObj {
        params: HashMap::new(),
        mode: ObjectMode::Trigger,
    };
    obj.params.insert(1, ObjParam::Number(1268.0));
    obj.params.insert(51, ObjParam::Group(target));
    obj.params
        .insert(57, ObjParam::Group(context.inner().group));
    globals.triggers.push(obj);
}

// #[allow(clippy::too_many_arguments)]
// fn macro_call(
//     code: &Code,
//     func: usize,
//     i: usize,
//     id: &u16,
//     m: &Macro,
//     base: &StoredValue,
//     globals: &mut Globals,
//     context: &mut FullContext,
// ) -> Option<Result<(), RuntimeError>> {
//     macro_rules! pop_deep_clone {
//         () => {{
//             let val = globals.memory[context.inner().stack.pop().unwrap()].clone();
//             val.deep_clone(globals)
//         }};
//         (Store) => {{
//             globals.key_deep_clone(context.inner().stack.pop().unwrap())
//         }};
//     }

//     let param_spans = code.macro_arg_spans.get(&(func, i)).unwrap();
//     let param_list = code.name_sets.get(*id);
//     let mut param_map = AHashMap::new();
//     let mut params = vec![];
//     let mut named_params = vec![];
//     for (name, param_span) in param_list.iter().zip(param_spans) {
//         if name.is_empty() {
//             params.push((pop_deep_clone!(), param_span));
//         } else {
//             if let Some(p) = m.args.iter().position(|MacroArg { name: s, .. }| s == name) {
//                 param_map.insert(name.clone(), p);
//             } else {
//                 return Some(Err(RuntimeError::UndefinedArgument {
//                     name: name.into(),
//                     macr: base.clone(),
//                     area: code.make_area(*param_span),
//                 }));
//             }
//             named_params.push((name.clone(), pop_deep_clone!(), param_span));
//         }
//     }
//     if params.len() > m.args.len() {
//         let call_span = code.get_bytecode_span(func, i);
//         return Some(Err(RuntimeError::TooManyArguments {
//             expected: m.args.len(),
//             provided: params.len(),
//             call_area: code.make_area(call_span),
//             func: base.clone(),
//         }));
//     }
//     let mut arg_fill = m
//         .args
//         .iter()
//         .map(
//             |MacroArg {
//                  pattern, default, ..
//              }| { (pattern.clone(), default.map(|id| globals.deep_clone(id))) },
//         )
//         .collect::<Vec<_>>();
//     params.reverse();
//     named_params.reverse();
//     for (i, (val, param_span)) in params.into_iter().enumerate() {
//         if let Some(pat) = &arg_fill[i].0 {
//             if !value_ops::matches_pat(&val.value, &pat.0) {
//                 return Some(Err(RuntimeError::PatternMismatch {
//                     v: val,
//                     pat: pat.clone(),
//                     area: code.make_area(*param_span),
//                 }));
//             }
//         }
//         arg_fill[i].1 = Some(val);
//     }
//     for (name, val, param_span) in named_params.into_iter() {
//         let arg_pos = param_map[&name];
//         if let Some(pat) = &arg_fill[arg_pos].0 {
//             if !value_ops::matches_pat(&val.value, &pat.0) {
//                 return Some(Err(RuntimeError::PatternMismatch {
//                     v: val,
//                     pat: pat.clone(),
//                     area: code.make_area(*param_span),
//                 }));
//             }
//         }
//         arg_fill[arg_pos].1 = Some(val);
//     }
//     for ((_, arg), MacroArg { name, area, .. }) in arg_fill.iter().zip(&m.args) {
//         if arg.is_none() {
//             let call_area = code.get_bytecode_span(func, i);
//             return Some(Err(RuntimeError::ArgumentNotSatisfied {
//                 arg_name: name.clone(),
//                 call_area: code.make_area(call_area),
//                 arg_area: area.clone(),
//             }));
//         }
//     }
//     context.inner().push_vars(
//         &code.bytecode_funcs[m.func_id].scoped_var_ids,
//         code,
//         globals,
//     );
//     context
//         .inner()
//         .push_vars(&code.bytecode_funcs[m.func_id].capture_ids, code, globals);
//     for ((_, arg), id) in arg_fill
//         .into_iter()
//         .zip(&code.bytecode_funcs[m.func_id].arg_ids)
//     {
//         context.inner().set_var(*id, arg.unwrap(), globals);
//     }
//     for (v, id) in m
//         .capture
//         .iter()
//         .zip(&code.bytecode_funcs[m.func_id].capture_ids)
//     {
//         context.inner().replace_var(*id, *v);
//     }
//     globals.call_counter.0 += 1;
//     context.inner().frames.push(Frame {
//         position: (m.func_id, 0),
//         call_id: globals.call_counter,
//         block_stack: vec![],
//     });
//     globals.calls.insert(globals.call_counter);
//     None
// }
