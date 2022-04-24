use crate::value::{Value, Error};

pub trait FromValue: Clone {
    fn from_value(val: Value) -> Result<Self, Error>;
}

impl FromValue for Value {
    fn from_value(val: Value) -> Result<Self, Error> {
        Ok(val)
    } 
}

impl FromValue for bool {
    fn from_value(val: Value) -> Result<Self, Error> {
        if let Value::Bool(b) = val {
            Ok(b)
        } else {
            Err(format!("type '{}' can't be converted to 'bool'", val.type_name()))
        }
    } 
}

macro_rules! value_to_num {
    ($($n:ty)*) => {
        $(
            impl FromValue for $n {
                fn from_value(val: Value) -> Result<Self, Error> {
                    if let Value::Number(n) = val {
                        if ((<$n>::MIN as f64)..(<$n>::MAX as f64)).contains(&n) {
                            Ok(n as $n)
                        } else {
                            panic!("cannot cannot cast 'f64' to '{}'! (value '{}' too large for '{}')", stringify!($n), n, stringify!($n))
                        }
                    } else {
                        Err(format!("type '{}' can't be converted to 'number'!", val.type_name()))
                    }
                }
            }
        )*
    };
}
value_to_num! { u8 u16 u32 u64 i16 i32 i64 f32 f64 }

pub trait FromValueList {
    fn from_value_list(values: &[Value]) -> Result<Self, Error>
        where Self: Sized;
}


macro_rules! tuple_value_list {

    ( $first:ident $( $name:ident )* ) => {
        tuple_value_list!( 0usize; $( $name )* );
    };

    ( $count:expr ; $first:ident $( $name:ident )* ) => {
        impl<
            $(
                $name: FromValue,
            )*
        > FromValueList for (
            $(
                $name,
            )*
        ) {
            fn from_value_list(values: &[Value]) -> Result<Self, Error>
                where Self: Sized
            {
                Ok((
                    $(
                        $name::from_value(values[$count])?,
                    )*
                ))
            }
        }

        tuple_value_list!( $count + 1usize ; $( $name )* );
    }; 

    ( $count:expr ; ) => {};
}

tuple_value_list! { A B C D E F G H I J K L M N O P Q R S T U V W X Y Z }