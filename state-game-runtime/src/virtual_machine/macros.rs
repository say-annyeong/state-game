#[macro_export]
macro_rules! define_function_registry {
    (
        $vis:vis enum $enum_name:ident;
        $vis_reg:vis const $registry_name:ident;

        $(
            $function_name:ident => {
                inputs: [$($input:expr),* $(,)?],
                output: $output:expr
            }
        ),* $(,)?
    ) => {
        #[repr(usize)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $enum_name {
            $(
                $function_name,
            )*
        }

        impl $enum_name {
            pub const COUNT: usize = [
                $(
                    stringify!($function_name)
                ),*
            ].len();
        }

        $vis_reg const $registry_name: FunctionRegistry<{ $enum_name::COUNT }> =
            FunctionRegistry {
                functions: [
                    $(
                        FunctionSignature {
                            inputs: &[
                                $($input),*
                            ],
                            output: $output,
                        },
                    )*
                ],
            };
    };
}