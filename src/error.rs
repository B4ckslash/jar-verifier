use std::fmt::Display;

macro_rules! define_errcodes {
    [ $( $name:ident : $class:ty ),+ ] => {
        #[derive(Debug)]
        pub enum Error {
            $(
                $name($class),
            )+
        }

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            $(
                Error::$name(ref err) => write!(f, "{}: {}", stringify!($name), err),
            )+
        }
    }
}
        impl std::error::Error for Error {
            fn cause(&self) -> Option<&dyn std::error::Error> {
                match *self {
                    $(
                        Error::$name(ref err) => Some(err),
                    )+
                }
            }
        }

        $(
            impl From<$class> for Error {
                fn from(e: $class) -> Self {
                    Error::$name(e)
                }
            }
        )+
    };
}

define_errcodes![
    ParsingError: java_class::error::Error
];
