macro_rules! define_ast {
    (
        $enum_name:ident {
            $(
                $struct_name:ident {
                    $( $field_name:ident : $field_type:ty ),+ $(,)?
                }
            ),* $(,)?
        }
    ) => {
        // Generate the structs
        $(
            #[derive(Debug, PartialEq)]
            pub(crate) struct $struct_name {
                $( pub(crate) $field_name: $field_type, )+
            }
        )*

        // Generate the wrapping enum
        #[derive(Debug, PartialEq)]
        pub(crate) enum $enum_name {
            $(
                $struct_name(Box<$struct_name>),
            )*
        }
    };
}
