#[doc(hidden)]
#[macro_export]
macro_rules! __asm_dedup_operands {
    ([$($ops:tt)*]) => {

        $($ops)*
    };
}

/// Helper macros receive: `helper!(args @collector STATE)` and must call:
#[macro_export]
macro_rules! asm_block {
    ($($tt:tt)*) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings []
            @auto_operands []
            @manual_operands []
            @options []
            @input [$($tt)*]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __asm_block_collect {
    (
        @strings []
        @auto_operands []
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input []
    ) => {
        ::core::arch::naked_asm!(
            "",
            $($manual)*
            $($opts)*
        )
    };

    (
        @strings []
        @auto_operands [$($auto:tt)+]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input []
    ) => {
        ::core::arch::naked_asm!(
            "",
            $($auto)+
            $($manual)*
            $($opts)*
        )
    };

    (
        @strings [$($s:expr,)+]
        @auto_operands []
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input []
    ) => {
        ::core::arch::naked_asm!(
            concat!($($s, "\n"),*),
            $($manual)*
            $($opts)*
        )
    };

    (
        @strings [$($s:expr,)+]
        @auto_operands [$($auto:tt)+]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input []
    ) => {
        ::core::arch::naked_asm!(
            concat!($($s, "\n"),*),
            $($auto)+
            $($manual)*
            $($opts)*
        )
    };

    (
        (
            @strings [$($s:expr,)*]
            @auto_operands [$($auto:tt)*]
            @manual_operands [$($manual:tt)*]
            @options [$($opts:tt)*]
            @input [$($rest:tt)*]
        )
        @new_lines   [$($new_s:expr,)*]
        @new_operands[$($new_ops:tt)*]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)* $($new_s,)*]
            @auto_operands [$($auto)* $($new_ops)*]
            @manual_operands [$($manual)*]
            @options [$($opts)*]
            @input [$($rest)*]
        }
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [$mac:ident ! ( $($inner:tt)* ), $($rest:tt)*]
    ) => {
        $mac!(
            $($inner)*
            @collector
            (
                @strings [$($s,)*]
                @auto_operands [$($auto)*]
                @manual_operands [$($manual)*]
                @options [$($opts)*]
                @input [$($rest)*]
            )
        )
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [$mac:ident ! ( $($inner:tt)* )]
    ) => {
        $mac!(
            $($inner)*
            @collector
            (
                @strings [$($s,)*]
                @auto_operands [$($auto)*]
                @manual_operands [$($manual)*]
                @options [$($opts)*]
                @input []
            )
        )
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [$name:ident = const $val:expr, $($rest:tt)*]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)*]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)* $name = const $val,]
            @options [$($opts)*]
            @input [$($rest)*]
        }
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [$name:ident = const $val:expr]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)*]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)* $name = const $val,]
            @options [$($opts)*]
            @input []
        }
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [$name:ident = sym $val:path, $($rest:tt)*]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)*]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)* $name = sym $val,]
            @options [$($opts)*]
            @input [$($rest)*]
        }
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [$name:ident = sym $val:path]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)*]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)* $name = sym $val,]
            @options [$($opts)*]
            @input []
        }
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [options($($o2:tt)*), $($rest:tt)*]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)*]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)*]
            @options [$($opts)* options($($o2)*),]
            @input [$($rest)*]
        }
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [options($($o2:tt)*)]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)*]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)*]
            @options [$($opts)* options($($o2)*),]
            @input []
        }
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [clobber_abi($($c:tt)*), $($rest:tt)*]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)*]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)*]
            @options [$($opts)* clobber_abi($($c)*),]
            @input [$($rest)*]
        }
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [clobber_abi($($c:tt)*)]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)*]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)*]
            @options [$($opts)* clobber_abi($($c)*),]
            @input []
        }
    };

    // String literals and expressions (must be last - :expr is greedy)

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [$e:expr, $($rest:tt)*]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)* $e,]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)*]
            @options [$($opts)*]
            @input [$($rest)*]
        }
    };

    (
        @strings [$($s:expr,)*]
        @auto_operands [$($auto:tt)*]
        @manual_operands [$($manual:tt)*]
        @options [$($opts:tt)*]
        @input [$e:expr]
    ) => {
        ::zeroos_macros::__asm_block_collect! {
            @strings [$($s,)* $e,]
            @auto_operands [$($auto)*]
            @manual_operands [$($manual)*]
            @options [$($opts)*]
            @input []
        }
    };
}

#[macro_export]
macro_rules! define_register_helpers {
    ($struct_type:path, $store_mnemonic:literal, $load_mnemonic:literal) => {
        macro_rules! store {
            ($r:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($r),
                                    ", {", stringify!([<s_ $r>]), "}(sp)"),
                        ]
                        @new_operands [
                            [<s_ $r>] = const memoffset::offset_of!($struct_type, $r),
                        ]
                    )
                }
            };

            ($r:ident, $f:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($r),
                                    ", {", stringify!([<s_ $f>]), "}(sp)"),
                        ]
                        @new_operands [
                            [<s_ $f>] = const memoffset::offset_of!($struct_type, $f),
                        ]
                    )
                }
            };

            ($r:ident, $f:ident, $base:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($r),
                                    ", {", stringify!([<s_ $f>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<s_ $f>] = const memoffset::offset_of!($struct_type, $f),
                        ]
                    )
                }
            };
        }

        macro_rules! load {
            ($r:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($r),
                                    ", {", stringify!([<l_ $r>]), "}(a0)"),
                        ]
                        @new_operands [
                            [<l_ $r>] = const memoffset::offset_of!($struct_type, $r),
                        ]
                    )
                }
            };

            ($r:ident, $f:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($r),
                                    ", {", stringify!([<l_ $f>]), "}(a0)"),
                        ]
                        @new_operands [
                            [<l_ $f>] = const memoffset::offset_of!($struct_type, $f),
                        ]
                    )
                }
            };

            ($r:ident, $f:ident, $base:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($r),
                                    ", {", stringify!([<l_ $f>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<l_ $f>] = const memoffset::offset_of!($struct_type, $f),
                        ]
                    )
                }
            };
        }
    };
}
