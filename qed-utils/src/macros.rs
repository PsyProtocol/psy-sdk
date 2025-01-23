use std::convert::AsMut;
use std::convert::AsRef;

#[macro_export]
macro_rules! impl_ref {
    ($enum_name:ident, $($variant:ident => $type:ty),*) => {
        $(
            impl AsRef<$type> for $enum_name {
                fn as_ref(&self) -> &$type {
                    match self {
                        Self::$variant(v) => v,
                        _ => panic!(concat!("Not a ", stringify!($variant), " type"))
                    }
                }
            }

            impl AsMut<$type> for $enum_name {
                fn as_mut(&mut self) -> &mut $type {
                    match self {
                        Self::$variant(v) => v,
                        _ => panic!(concat!("Not a ", stringify!($variant), " type"))
                    }
                }
            }
        )*
    };
}
