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
            #[derive(Clone, Debug, PartialEq)]
            pub(crate) struct $struct_name {
                $( pub(crate) $field_name: $field_type, )+
            }

            impl $struct_name {
                pub(crate) fn lift( $($field_name: $field_type, )+ ) -> $enum_name {
                    $enum_name::$struct_name(Box::new($struct_name{ $($field_name, )+ }))
                }
            }
        )*

        // Generate the wrapping enum
        #[derive(Clone, Debug, PartialEq)]
        pub(crate) enum $enum_name {
            $(
                $struct_name(Box<$struct_name>),
            )*
        }
    };
}
