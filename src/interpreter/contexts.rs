use crate::compiler::compiler::{Code, InstrNum};

use super::{
    interpreter::{CallId, Globals, StoredValue, ValueKey},
    value::{Id, ValueIter},
};

#[derive(Debug, Clone)]
pub struct Context {
    pub group: Id,
    pub id: String,
    pub pos: Vec<(usize, usize, CallId)>,
    pub vars: Vec<Vec<ValueKey>>,
    pub stack: Vec<ValueKey>,
    pub iter_stack: Vec<ValueIter>,
}
impl Context {
    pub fn new(var_count: usize) -> Self {
        Self {
            group: Id::Specific(0),
            id: "O".into(),
            vars: vec![vec![]; var_count],
            pos: vec![(0, 0, CallId(0))],
            stack: vec![],
            iter_stack: vec![],
        }
    }
    pub fn pos(&mut self) -> &mut (usize, usize, CallId) {
        self.pos.last_mut().unwrap()
    }
    pub fn advance_to(&mut self, code: &Code, i: usize, globals: &mut Globals) {
        self.pos().1 = i;
        let (func, i, call_id) = *self.pos();
        if i >= code.bytecode_funcs[func].instructions.len() {
            if !globals.calls.contains(&call_id) {
                self.yeet();
                return;
            }
            self.return_out(code, globals);
        }
    }
    pub fn advance_by(&mut self, code: &Code, n: usize, globals: &mut Globals) {
        let (_, i, _) = *self.pos();
        self.advance_to(code, i + n, globals)
    }

    pub fn set_group(&mut self, group: Id) {
        self.group = group;
    }
    pub fn return_out(&mut self, code: &Code, globals: &mut Globals) {
        let (func, _, _) = *self.pos();
        self.pos.pop();
        self.pop_vars(&code.bytecode_funcs[func].scoped_var_ids);
        self.pop_vars(&code.bytecode_funcs[func].capture_ids);
        if !self.pos.is_empty() {
            self.advance_by(code, 1, globals);
        }
    }
    pub fn yeet(&mut self) {
        self.pos = vec![];
    }

    pub fn push_vars(&mut self, vars: &[InstrNum], code: &Code, globals: &mut Globals) {
        for i in vars {
            self.vars[*i as usize].push(globals.memory.insert(code.dummy_value()));
        }
    }
    pub fn pop_vars(&mut self, vars: &[InstrNum]) {
        for i in vars {
            self.vars[*i as usize].pop();
        }
    }

    pub fn set_var(&mut self, id: InstrNum, var: StoredValue, globals: &mut Globals) {
        let key = self.get_var(id);
        globals.memory[key] = var;
        // *self.vars[id as usize].last_mut().unwrap() = k
    }
    pub fn replace_var(&mut self, id: InstrNum, k: ValueKey) {
        *self.vars[id as usize].last_mut().unwrap() = k
    }
    pub fn get_var(&self, id: InstrNum) -> ValueKey {
        *self.vars[id as usize].last().unwrap()
    }
    pub fn clone_context(&self, globals: &mut Globals) -> Context {
        let mut vars = vec![];
        for i in &self.vars {
            let var = i
                .iter()
                .map(|k| {
                    let value = globals.memory[*k].clone();
                    globals.memory.insert(value)
                })
                .collect::<Vec<_>>();
            vars.push(var);
        }
        let stack = self
            .stack
            .iter()
            .map(|p| {
                let value = globals.memory[*p].clone();
                globals.memory.insert(value)
            })
            .collect::<Vec<_>>();

        Context {
            group: self.group,
            id: self.id.clone(),
            pos: self.pos.clone(),
            vars,
            stack,
            iter_stack: self.iter_stack.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FullContext {
    Single(Context),
    Split(Box<FullContext>, Box<FullContext>),
}

impl FullContext {
    pub fn single(var_count: usize) -> Self {
        Self::Single(Context::new(var_count))
    }

    pub fn remove_finished(&mut self) -> bool {
        let stack = Self::stack(&mut self.iter().filter_map(|c| {
            if c.inner().pos.is_empty() {
                None
            } else {
                Some(c.clone())
            }
        }));
        if let Some(c) = stack {
            *self = c;
            true
        } else {
            false
        }
    }

    pub fn stack(list: &mut impl Iterator<Item = Self>) -> Option<Self> {
        let first = list.next()?;
        match Self::stack(list) {
            Some(second) => Some(FullContext::Split(first.into(), second.into())),
            None => Some(first),
        }
    }

    pub fn inner(&mut self) -> &mut Context {
        if let FullContext::Single(c) = self {
            c
        } else {
            panic!("Tried to take inner context of split context")
        }
    }
    // pub fn inner_mut(&mut self) -> &mut Context {
    //     if let FullContext::Single(c) = self {
    //         c
    //     } else {
    //         panic!("Tried to take inner context of split context")
    //     }
    // }

    pub fn split_context(&mut self, globals: &mut Globals) {
        let mut a = self.inner().clone();
        let mut b = a.clone_context(globals);
        a.id += "a";
        b.id += "b";
        let a = FullContext::Single(a);
        let b = FullContext::Single(b);
        *self = FullContext::Split(Box::new(a), Box::new(b));
    }

    pub fn iter(&mut self) -> ContextIter {
        ContextIter::new(self)
    }
}

pub struct ContextIter<'a> {
    current_node: Option<&'a mut FullContext>,
    right_nodes: Vec<&'a mut FullContext>,
}
impl<'a> ContextIter<'a> {
    fn new(node: &'a mut FullContext) -> Self {
        let mut iter = Self {
            current_node: None,
            right_nodes: vec![],
        };
        iter.add_left_subtree(node);
        iter
    }
    fn add_left_subtree(&mut self, mut node: &'a mut FullContext) {
        loop {
            match node {
                FullContext::Split(left, right) => {
                    self.right_nodes.push(right);
                    node = left;
                }
                single => {
                    self.current_node = Some(single);
                    break;
                }
            }
        }
    }
}

impl<'a> Iterator for ContextIter<'a> {
    type Item = &'a mut FullContext;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.current_node.take();

        if let Some(r) = self.right_nodes.pop() {
            self.add_left_subtree(r);
        }

        match result {
            Some(c) => {
                // if c.inner().broken.is_some() {
                //     self.next()
                // } else {
                Some(c)
                // }
            }
            None => None,
        }
    }
}
