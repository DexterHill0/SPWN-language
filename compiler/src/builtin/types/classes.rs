use std::any::{TypeId, type_name, Any};
use std::marker::PhantomData;
use std::sync::Arc;
use std::collections::HashMap;

use crate::globals::Globals;
use crate::value::Value;
use crate::to_value::{ToValue, ToValueResult};
use crate::from_value::{FromValueList};
use crate::builtin::types::methods::{Function, Method};

pub type Error = String;


type StaticMethodType<T> = Arc<dyn Fn(Vec<Value>) -> Result<T, Error> + Send + Sync>;
type SelfMethodType<T> =
    Arc<dyn Fn(&Instance, Vec<Value>, &mut Globals) -> Result<T, Error> + Send + Sync>;

////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct SelfMethod(SelfMethodType<Value>);

impl SelfMethod {
    pub fn new<T, F, Args>(f: F) -> Self
        where
            Args: FromValueList,
            F: Method<T, Args>,
            F::Result: ToValueResult,
            T: 'static,
    {
        Self(Arc::new(
            move |instance: &Instance, args: Vec<Value>, globals: &mut Globals| {
                let instance = instance
                    .downcast(Some(globals));

                let args = Args::from_value_list(&args);

                instance.and_then(|i| args.map(|a| (i, a)))
                    .and_then(|(instance, args)| f.invoke(instance, args).to_value_result())
            },
        ))
    }

    pub fn from_static_method(name: String, method: Option<StaticMethod>) -> Self {
        Self(Arc::new(
            move |_: &Instance, args: Vec<Value>, _: &mut Globals| {
                method.as_ref().ok_or(format!("Static method '{}' is undefined!", name))?.invoke(args)
            },
        ))
    }

    pub fn invoke(
        &self,
        instance: &Instance,
        args: Vec<Value>,
        globals: &mut Globals,
    ) -> Result<Value, Error> {
        self.0(instance, args, globals)
    }
}


type SelfMethods = HashMap<&'static str, SelfMethod>;
////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct StaticMethod(StaticMethodType<Value>);

impl StaticMethod {
    pub fn new<F, Args>(f: F) -> Self
    where
        Args: FromValueList,
        F: Function<Args>,
        F::Result: ToValueResult,
    {
        Self(Arc::new(move |args: Vec<Value>| {
            Args::from_value_list(&args).and_then(|args| f.invoke(args).to_value_result())
        }))
    }

    pub fn invoke(&self, args: Vec<Value>) -> Result<Value, Error> {
        self.0(args)
    }
}


type StaticMethods = HashMap<&'static str, StaticMethod>;
////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////
#[derive(Clone)]
pub struct AttributeGetter(
    Arc<dyn Fn(&Instance, &mut Globals) -> Result<Value, Error> + Send + Sync>,
);

impl AttributeGetter {
    pub fn new<T, F, R>(f: F) -> Self
        where
            T: 'static,
            F: Fn(&T) -> R + Send + Sync + 'static,
            R: ToValueResult,
    {
        Self(Arc::new(move |instance, globals: &mut Globals| {
            let instance = instance
                .downcast(Some(globals));
            instance.map(&f).and_then(|v| v.to_value_result())
        }))
    }

    pub fn invoke(&self, instance: &Instance, globals: &mut Globals) -> Result<Value, Error> {
        self.0(instance, globals)
    }
}

type Attributes = HashMap<&'static str, AttributeGetter>;
////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct Constructor(StaticMethodType<Instance>);

impl Constructor {
    pub fn new<Args, F>(f: F) -> Self
        where
            Args: FromValueList,
            F: Function<Args>,
            F::Result: Send + Sync + 'static,
    {
        Constructor(Arc::new(move |args: Vec<Value>| {
            Args::from_value_list(&args).map(|args| Instance::new(f.invoke(args)))
        }))
    }

    pub fn invoke(&self, args: Vec<Value>) -> Result<Instance, Error> {
        self.0(args)
    }
}
////////////////////////////////////////////////////////////////////



////////////////////////////////////////////////////////////////////
pub struct Type {
    pub name: String,
    pub type_id: TypeId,
    constructor: Option<Constructor>,
    attributes: Attributes,
    self_methods: SelfMethods,
    static_methods: StaticMethods,
}

impl Type {
    pub fn call_static(&self, name: &str, args: Vec<Value>) -> Result<Value, Error> {
        let attr =
            self.static_methods
                .get(name)
                .ok_or_else(|| format!("Static method '{}' is undefined!", name))?;

        attr.clone().invoke(args)
    }

    pub fn get_self_method(&self, name: &str) -> Option<SelfMethod> {
        // if the self method doesnt exist check if it's a static method as they also can be called from `self.`
        return if let Some(method) = self.self_methods.get(name).cloned() {
            Some(method)
        } else {
            Some(SelfMethod::from_static_method(name.to_string(), self.static_methods.get(name).cloned()))
        }

    }
}

pub struct TypeBuilder<T> {
    typ: Type,
    ty: PhantomData<T>,
}

impl<T> TypeBuilder<T> 
    where T: 'static
{
    pub fn name(name: &'static str) -> Self {
        Self {
            typ: Type {
                name: name.to_string(),
                constructor: None,
                attributes: Attributes::new(),
                type_id: TypeId::of::<T>(),
                static_methods: StaticMethods::new(),
                self_methods: SelfMethods::new(),
            },
            ty: std::marker::PhantomData,
        }
    }

    pub fn add_attribute<F, R>(mut self, name: &'static str, f: F) -> Self
        where
            F: Fn(&T) -> R + Send + Sync + 'static,
            R: ToValue,
            T: 'static,
    {
        self.typ.attributes.insert(name, AttributeGetter::new(f));
        self
    }
    
    pub fn set_constructor<F, Args>(mut self, f: F) -> Self
        where
            F: Function<Args, Result = T>,
            T: Send + Sync,
            Args: FromValueList,
    {
        self.typ.constructor = Some(Constructor::new(f));
        self
    }

    pub fn add_static_method<F, Args, R>(mut self, name: &'static str, f: F) -> Self
        where
            F: Function<Args, Result = R>,
            Args: FromValueList,
            R: ToValueResult + 'static,
    {
        self.typ.static_methods.insert(name, StaticMethod::new(f));
        self
    }

    pub fn add_self_method<F, Args, R>(mut self, name: &'static str, f: F) -> Self
        where
            Args: FromValueList,
            F: Method<T, Args, Result = R>,
            R: ToValueResult + 'static,
    {
        self.typ.self_methods.insert(name, SelfMethod::new(f));
        self
    }

    pub fn finish(self) -> Type {
        self.typ
    }
}
////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct Instance {
    inner: Arc<dyn Any + Send + Sync>,
    debug_type_name: &'static str,
}


impl Instance {
    pub fn of(typ: &Type, fields: Vec<Value>) -> Result<Self, Error> {
        if let Some(ctor) = &typ.constructor {
            ctor.invoke(fields)
        } else {
            return Err(format!("Class '{}' has no constructor!", typ.name));
        }
    }

    pub fn new<T: Send + Sync + 'static>(instance: T) -> Self {
        Self {
            inner: Arc::new(instance),
            debug_type_name: type_name::<T>(),
        }
    }

    pub fn instance_of<T>(&self, typ: &Type) -> bool {
        self.type_id() == typ.type_id
    }

    pub fn class<'a>(&self, globals: &'a Globals) -> Result<&'a Type, Error> {
        globals.types.get(&self.type_id())
            .ok_or_else(|| format!("Class '{:?}' is undefined!", self.debug_type_name))
            
    }

    pub fn name<'a>(&self, globals: &'a Globals) -> &'a str {
        self.class(globals)
            .map(|class| class.name.as_ref())
            .unwrap_or_else(|_| self.debug_type_name)
    }

    pub fn get_attr(&self, name: &str, globals: &mut Globals) -> Result<Value, Error> {
        let attr = self
            .class(globals)
            .and_then(|c| {
                c.attributes.get(name).ok_or_else(|| format!("Attribute '{}' is undefined!", name))
            })?
            .clone();
        attr.invoke(self, globals)

    }

    pub fn call_self(&self, name: &str, args: Vec<Value>, globals: &mut Globals) -> Result<Value, Error> {
        let method = self.class(globals).and_then(|c| {
            c.get_self_method(name).ok_or_else(|| format!("Self method '{}' is undefined!", name))
        })?;
        method.invoke(self, args, globals)
    }

    pub fn downcast<T: 'static>(
        &self,
        globals: Option<&mut Globals>,
    ) -> Result<&T, Error> {
        let name = globals.as_ref()
            .map(|g| self.name(g).to_owned())
            .unwrap_or_else(|| self.debug_type_name.to_owned());

        let expected_name = globals.as_ref()
            .and_then(|g| g.types.get(&TypeId::of::<T>())
                .map(|class| class.name.clone())
            ).unwrap_or_else(|| self.debug_type_name.to_owned());

        self.inner
            .as_ref()
            .downcast_ref()
            .ok_or_else(|| format!("Expected type '{}', got '{}'!", expected_name, name))
    }

    pub fn raw<T: Send + Sync + 'static>(&self) -> Result<&T, Error> {
        self.downcast::<T>(None)
    }
    
    pub fn type_id(&self) -> TypeId {
        self.inner.as_ref().type_id()
    }
}