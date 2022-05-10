use std::any::{type_name, Any};
use std::marker::PhantomData;
use std::sync::Arc;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::globals::Globals;
use crate::value::Value;
use crate::to_value::{ToValue, ToValueResult};
use crate::from_value::{FromValueList};
use crate::builtin::types::methods::{Function, Method};

pub type Error = String;


// hashing used instead of `TypeId` as it needs to be an id unique per unique instance rather rather than type, i.e.
pub type HashId = u64;

trait GetHash {
    fn hash_id(&self) -> HashId 
        where Self: Hash + Sized
    {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}
impl<T> GetHash for T where T: ?Sized + Any {}

fn hash_type_name<T>() -> HashId 
    where T: Hash,
{
    let mut hasher = DefaultHasher::new();
    hasher.write(&type_name::<T>().as_bytes());
    hasher.finish()
}

type StaticMethodType<T> = Arc<dyn Fn(Vec<Value>) -> Result<T, Error> + Send + Sync>;
type SelfMethodType<T> =
    Arc<dyn Fn(&Instance, Vec<Value>, &mut Globals) -> Result<T, Error> + Send + Sync>;



// `SelfMethod` i.e. a instance method (where `self` is the first argument)
#[derive(Clone)]
pub struct SelfMethod(SelfMethodType<Value>);

impl SelfMethod {

    pub fn new<T, F, Args>(f: F) -> Self
        where
            Args: FromValueList,
            F: Method<T, Args>,
            F::Result: ToValueResult,
            T: Hash + 'static,
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

    pub fn from_static_method(method: StaticMethod) -> Self {
        Self(Arc::new(
            move |_: &Instance, args: Vec<Value>, _: &mut Globals| {
                method.invoke(args)
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
type SelfMethods = HashMap<String, SelfMethod>;


// `StaticMethod` where `self` isnt the first argument
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
type StaticMethods = HashMap<String, StaticMethod>;


#[derive(Clone)]
pub struct AttributeGetter(
    Arc<dyn Fn(&Instance, &mut Globals) -> Result<Value, Error> + Send + Sync>,
);

impl AttributeGetter {
    pub fn new<T, F, R>(f: F) -> Self
        where
            T: Hash + 'static,
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

type Attributes = HashMap<String, AttributeGetter>;
////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct Constructor(StaticMethodType<Instance>);

impl Constructor {
    pub fn new<Args, F>(f: F) -> Self
        where
            Args: FromValueList,
            F: Function<Args>,
            F::Result: Hash + Send + Sync + 'static,
    {
        Constructor(Arc::new(move |args: Vec<Value>| {
            Args::from_value_list(&args).map(|args| {
                let s = f.invoke(args);
                let id = (&s).hash_id();
                Instance::new(s, id)
            })
        }))
    }

    pub fn invoke(&self, args: Vec<Value>) -> Result<Instance, Error> {
        self.0(args)
    }
}


#[derive(Clone)]
pub struct Type {
    pub name: String,
    pub type_id: HashId,
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

    pub fn get_self_method(&self, name: String) -> Result<SelfMethod, Error> {
        // if the self method doesnt exist check if it's a static method as they also can be called from `self.`
        if let Some(method) = self.self_methods.get(&name).cloned() {
            return Ok(method)
        } 
        if let Some(method) = self.static_methods.get(&name).cloned() {
            return Ok(SelfMethod::from_static_method(method))
        }
        Err(format!("Self method '{}' is undefined!", name))
    }
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.name.as_bytes());
    }
}

pub struct TypeBuilder<T> {
    typ: Type,
    ty: PhantomData<T>,
}

impl<T> TypeBuilder<T> 
    where 
        T: 'static,
{
    pub fn name<U: ToString>(name: U) -> Self {
        let hashed = (&name.to_string()).hash_id();
        Self {
            typ: Type {
                name: name.to_string(),
                constructor: None,
                attributes: Attributes::new(),
                type_id: hashed,
                static_methods: StaticMethods::new(),
                self_methods: SelfMethods::new(),
            },
            ty: PhantomData,
        }
    }

    pub fn from(typ: Type) -> Self {
        Self {
            typ,
            ty: PhantomData,
        }
    }

    pub fn add_attribute<F, R, S>(mut self, name: S, f: F) -> Self
        where
            F: Fn(&T) -> R + Send + Sync + 'static,
            R: ToValue,
            T: Hash + 'static,
            S: ToString
    {
        self.typ.attributes.insert(name.to_string(), AttributeGetter::new(f));
        self
    }
    
    pub fn set_constructor<F, Args, R>(mut self, f: F) -> Self
        where
            F: Function<Args, Result = R>,
            T: Hash + Send + Sync,
            R: Hash + Send + Sync + 'static,
            Args: FromValueList,
    {
        self.typ.constructor = Some(Constructor::new(f));
        self
    }

    pub fn add_static_method<F, Args, R, S>(mut self, name: S, f: F) -> Self
        where
            F: Function<Args, Result = R>,
            Args: FromValueList,
            R: ToValueResult + 'static,
            S: ToString
    {
        self.typ.static_methods.insert(name.to_string(), StaticMethod::new(f));
        self
    }

    pub fn add_self_method<F, Args, R, S>(mut self, name: S, f: F) -> Self
        where
            Args: FromValueList,
            F: Method<T, Args, Result = R>,
            R: ToValueResult + 'static,
            T: Hash,
            S: ToString
    {
        self.typ.self_methods.insert(name.to_string(), SelfMethod::new(f));
        self
    }

    pub fn finish(self) -> Type {
        self.typ
    }
}
////////////////////////////////////////////////////////////////////



#[derive(Clone)]
pub struct Instance {
    inner: Arc<dyn Any + Send + Sync>,
    type_id: HashId,
    debug_type_name: &'static str,
}


impl Instance {
    pub fn of(typ: &Type, fields: Vec<Value>) -> Result<Self, Error> {
        if let Some(ctor) = &typ.constructor {
            ctor.invoke(fields)
        } else {
               Err(format!("Type '{}' has no constructor!", typ.name))
        }
    }

    pub fn new<T: Send + Sync + 'static>(instance: T, type_id: HashId) -> Self {
        Self {
            inner: Arc::new(instance),
            debug_type_name: type_name::<T>(),
            type_id,
        }
    }

    pub fn instance_of<T>(&self, typ: &Type) -> bool {
        self.type_id == typ.type_id
    }

    pub fn inner_type<'a>(&self, globals: &'a Globals) -> Result<&'a Type, Error> {
        globals.types.get(&self.type_id)
            .ok_or_else(|| format!("Type '{:?}' is undefined!", self.debug_type_name))
            
    }

    pub fn name<'a>(&self, globals: &'a Globals) -> &'a str {
        self.inner_type(globals)
            .map(|ty| ty.name.as_ref())
            .unwrap_or_else(|_| self.debug_type_name)
    }

    pub fn get_attr(&self, name: &str, globals: &mut Globals) -> Result<Value, Error> {
        let attr = self
            .inner_type(globals)
            .and_then(|c| {
                c.attributes.get(name).ok_or_else(|| format!("Attribute '{}' is undefined!", name))
            })?
            .clone();
        attr.invoke(self, globals)

    }

    pub fn call_self(&self, name: String, args: Vec<Value>, globals: &mut Globals) -> Result<Value, Error> {
        let method = self.inner_type(globals).and_then(|c| {
            c.get_self_method(name.clone())
        })?;
        method.invoke(self, args, globals)
    }

    pub fn downcast<T: Hash + 'static>(
        &self,
        globals: Option<&mut Globals>,
    ) -> Result<&T, Error> {
        let name = globals.as_ref()
            .map(|g| self.name(g).to_owned())
            .expect("tried to get inner type name of instance from a type that is not stored in globals!");

        let expected_name = globals.as_ref()
            .and_then(|g| g.types.get(&hash_type_name::<T>())
                .map(|ty| ty.name.clone())
            ).unwrap_or_else(|| self.debug_type_name.to_owned());

        self.inner
            .as_ref()
            .downcast_ref()
            .ok_or_else(|| format!("Expected type '{}', got '{}'!", expected_name, name))
    }

    pub fn raw<T: Hash + Send + Sync + 'static>(&self) -> Result<&T, Error> {
        self.downcast::<T>(None)
    }
}