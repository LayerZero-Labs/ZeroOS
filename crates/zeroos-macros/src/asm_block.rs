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
    ($store_mnemonic:literal, $load_mnemonic:literal) => {
        #[allow(unused_macros)]
        macro_rules! store {
            // Explicit struct type, implicit field name == register name
            ($struct_type:path, $r:ident @collector $state:tt) => {
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

            // Explicit struct type, implicit field name == register name, explicit base
            ($struct_type:path, $r:ident, $base:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($r),
                                    ", {", stringify!([<s_ $r>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<s_ $r>] = const memoffset::offset_of!($struct_type, $r),
                        ]
                    )
                }
            };

            // Explicit struct type + explicit field name + explicit base
            ($struct_type:path, $r:ident, $f:ident, $base:ident @collector $state:tt) => {
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

            // Value-first, brace-wrapped field-access style:
            // `store!(sp, {ThreadAnchor.user_sp}(tp))`
            //
            // We intentionally use `{Type.field}` to avoid `macro_rules!` restrictions
            // around `$path` fragments followed by `.`.
            //
            // Optional operand tag:
            // `store!(sp, {ThreadAnchor.user_sp}(tp) @tag)`
            // This forces a unique operand name even if the same store is emitted twice
            // in one `asm_block!` expansion.
            //
            // Shorthand when the struct field name matches the register name:
            // `store!(gp, {TrapFrame}(sp))` == `store!(gp, {TrapFrame.gp}(sp))`
            ($val:ident, { $struct_type:ident } ( $base:ident ) @ $tag:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($val),
                                    ", {", stringify!([<s_ $val _ $val _ $tag>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<s_ $val _ $val _ $tag>] = const memoffset::offset_of!($struct_type, $val),
                        ]
                    )
                }
            };
            ($val:ident, { $struct_type:ident } ( $base:ident ) @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($val),
                                    ", {", stringify!([<s_ $val _ $val>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<s_ $val _ $val>] = const memoffset::offset_of!($struct_type, $val),
                        ]
                    )
                }
            };
            ($val:ident, { $struct_type:ident . $field:ident } ( $base:ident ) @ $tag:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($val),
                                    ", {", stringify!([<s_ $field _ $val _ $tag>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<s_ $field _ $val _ $tag>] = const memoffset::offset_of!($struct_type, $field),
                        ]
                    )
                }
            };
            ($val:ident, { $struct_type:ident . $field:ident } ( $base:ident ) @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($val),
                                    ", {", stringify!([<s_ $field _ $val>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<s_ $field _ $val>] = const memoffset::offset_of!($struct_type, $field),
                        ]
                    )
                }
            };

            // Value-first, brace-wrapped indexed array field-access style:
            // `store!(t0, {ThreadAnchor.scratch[0]}(tp))`
            //
            // Optional operand tag:
            // `store!(t0, {ThreadAnchor.scratch[0]}(tp) @tag)`
            ($val:ident, { $struct_type:ident . $field:ident [ $idx:literal ] } ( $base:ident ) @ $tag:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($val),
                                    ", {", stringify!([<s_ $field _ $idx _ $val _ $tag>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<s_ $field _ $idx _ $val _ $tag>] = const (
                                memoffset::offset_of!($struct_type, $field)
                                    + ($idx as usize) * ::core::mem::size_of::<usize>()
                            ),
                        ]
                    )
                }
            };
            ($val:ident, { $struct_type:ident . $field:ident [ $idx:literal ] } ( $base:ident ) @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($store_mnemonic, " ", stringify!($val),
                                    ", {", stringify!([<s_ $field _ $idx _ $val>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<s_ $field _ $idx _ $val>] = const (
                                memoffset::offset_of!($struct_type, $field)
                                    + ($idx as usize) * ::core::mem::size_of::<usize>()
                            ),
                        ]
                    )
                }
            };

            // Note: no default struct type. Always pass the target struct explicitly:
            // e.g. `store!(TrapFrame, t0, sp)` or `store!(TrapFrame, t0)`.
        }

        #[allow(unused_macros)]
        macro_rules! load {
            // Explicit struct type, implicit field name == register name
            ($struct_type:path, $r:ident @collector $state:tt) => {
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

            // Explicit struct type, implicit field name == register name, explicit base
            ($struct_type:path, $r:ident, $base:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($r),
                                    ", {", stringify!([<l_ $r>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<l_ $r>] = const memoffset::offset_of!($struct_type, $r),
                        ]
                    )
                }
            };

            // Explicit struct type + explicit field name + explicit base
            ($struct_type:path, $r:ident, $f:ident, $base:ident @collector $state:tt) => {
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

            // Value-first, brace-wrapped field-access style:
            // `load!(t0, {ThreadAnchor.kstack_base}(tp))`
            //
            // Optional operand tag:
            // `load!(t0, {ThreadAnchor.kstack_base}(tp) @tag)`
            //
            // Shorthand when the struct field name matches the destination register name:
            // `load!(gp, {TrapFrame}(sp))` == `load!(gp, {TrapFrame.gp}(sp))`
            ($dst:ident, { $struct_type:ident } ( $base:ident ) @ $tag:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($dst),
                                    ", {", stringify!([<l_ $dst _ $dst _ $tag>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<l_ $dst _ $dst _ $tag>] = const memoffset::offset_of!($struct_type, $dst),
                        ]
                    )
                }
            };
            ($dst:ident, { $struct_type:ident } ( $base:ident ) @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($dst),
                                    ", {", stringify!([<l_ $dst _ $dst>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<l_ $dst _ $dst>] = const memoffset::offset_of!($struct_type, $dst),
                        ]
                    )
                }
            };
            ($dst:ident, { $struct_type:ident . $field:ident } ( $base:ident ) @ $tag:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($dst),
                                    ", {", stringify!([<l_ $field _ $dst _ $tag>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<l_ $field _ $dst _ $tag>] = const memoffset::offset_of!($struct_type, $field),
                        ]
                    )
                }
            };
            ($dst:ident, { $struct_type:ident . $field:ident } ( $base:ident ) @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($dst),
                                    ", {", stringify!([<l_ $field _ $dst>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<l_ $field _ $dst>] = const memoffset::offset_of!($struct_type, $field),
                        ]
                    )
                }
            };

            // Value-first, brace-wrapped indexed array field-access style:
            // `load!(t0, {ThreadAnchor.scratch[0]}(tp))`
            //
            // Optional operand tag:
            // `load!(t0, {ThreadAnchor.scratch[0]}(tp) @tag)`
            ($dst:ident, { $struct_type:ident . $field:ident [ $idx:literal ] } ( $base:ident ) @ $tag:ident @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($dst),
                                    ", {", stringify!([<l_ $field _ $idx _ $dst _ $tag>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<l_ $field _ $idx _ $dst _ $tag>] = const (
                                memoffset::offset_of!($struct_type, $field)
                                    + ($idx as usize) * ::core::mem::size_of::<usize>()
                            ),
                        ]
                    )
                }
            };
            ($dst:ident, { $struct_type:ident . $field:ident [ $idx:literal ] } ( $base:ident ) @collector $state:tt) => {
                paste::paste! {
                    ::zeroos_macros::__asm_block_collect!(
                        $state
                        @new_lines [
                            concat!($load_mnemonic, " ", stringify!($dst),
                                    ", {", stringify!([<l_ $field _ $idx _ $dst>]),
                                    "}(", stringify!($base), ")"),
                        ]
                        @new_operands [
                            [<l_ $field _ $idx _ $dst>] = const (
                                memoffset::offset_of!($struct_type, $field)
                                    + ($idx as usize) * ::core::mem::size_of::<usize>()
                            ),
                        ]
                    )
                }
            };

            // Note: no default struct type. Always pass the target struct explicitly:
            // e.g. `load!(TrapFrame, t0)` or `load!(TrapFrame, t0, a1)`.
        }
    };
}
