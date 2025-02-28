use std::ffi::CString;
use std::marker::PhantomData;

use sealed::sealed;

use crate::invocable::{GlobalMetadata, MethodMetadata};
use crate::systems::RttiSystemMut;
use crate::types::{CName, NativeClass};
use crate::{NativeRepr, RttiSystem, ScriptClass, class_kind};

/// A list of exports to register with the game.
#[derive(Debug)]
pub struct ExportList<H, T> {
    head: H,
    tail: T,
}

impl<H, T> ExportList<H, T> {
    /// Create a new `ExportList` with the given head and tail.
    pub const fn new(head: H, tail: T) -> Self {
        Self { head, tail }
    }
}

/// A trait for types to be exported to the game.
#[sealed]
pub trait Exportable {
    fn register(&self);
    fn post_register(&self);
}

#[sealed]
impl<H, T> Exportable for ExportList<H, T>
where
    H: Exportable,
    T: Exportable,
{
    #[inline]
    fn register(&self) {
        self.head.register();
        self.tail.register();
    }

    #[inline]
    fn post_register(&self) {
        self.head.post_register();
        self.tail.post_register();
    }
}

/// A type representing an empty list of exports.
#[derive(Debug)]
pub struct ExportNil;

#[sealed]
impl Exportable for ExportNil {
    #[inline]
    fn register(&self) {}

    #[inline]
    fn post_register(&self) {}
}

/// A single class export.
/// This can be used to define a custom class to be exported to the game.
/// This type should not be used for structs, use [`StructExport`] instead.
#[derive(Debug)]
pub struct ClassExport<C: 'static> {
    base: &'static str,
    methods: &'static [MethodMetadata<C>],
    static_methods: &'static [GlobalMetadata],
}

impl<C: ScriptClass> ClassExport<C> {
    pub fn builder() -> ClassExportBuilder<C> {
        ClassExportBuilder {
            base: "IScriptable",
            methods: &[],
            static_methods: &[],
        }
    }
}

#[sealed]
impl<C: Default + Clone + ScriptClass<Kind = class_kind::Native>> Exportable for ClassExport<C> {
    fn register(&self) {
        let mut rtti = RttiSystemMut::get();
        let name_cstr = CString::new(C::NAME).expect("name should be valid");
        let base = rtti
            .get_class(CName::new(self.base))
            .expect("base should exist");
        let handle = NativeClass::<C>::new_handle(&name_cstr, Some(base));
        rtti.register_class(handle);
    }

    fn post_register(&self) {
        let (converted_methods, converted_static_methods) = {
            let rtti_ro = RttiSystem::get();
            let class = rtti_ro
                .get_class(CName::new(C::NAME))
                .expect("class should exist");
            let converted_methods = self
                .methods
                .iter()
                .map(|m| m.to_rtti(class))
                .collect::<Vec<_>>();
            let converted_static_methods = self
                .static_methods
                .iter()
                .map(|m| m.to_rtti_static_method(class))
                .collect::<Vec<_>>();
            (converted_methods, converted_static_methods)
        };

        let mut rtti_rw = RttiSystemMut::get();
        let class = rtti_rw
            .get_class(CName::new(C::NAME))
            .expect("class should exist");

        for method in converted_methods {
            class.add_method(method);
        }
        for static_method in converted_static_methods {
            class.add_static_method(static_method);
        }
    }
}

/// A builder for [`ClassExport`].
#[derive(Debug)]
pub struct ClassExportBuilder<C: 'static> {
    base: &'static str,
    methods: &'static [MethodMetadata<C>],
    static_methods: &'static [GlobalMetadata],
}

impl<C> ClassExportBuilder<C> {
    /// Set the base class of the class to be exported.
    /// This is set to `IScriptable` by default.
    pub const fn base(mut self, base: &'static str) -> Self {
        self.base = base;
        self
    }

    /// Set the methods of the class to be exported.
    /// See the [`methods!`](crate::methods) macro for a convenient way to define methods.
    pub const fn methods(mut self, methods: &'static [MethodMetadata<C>]) -> Self {
        self.methods = methods;
        self
    }

    /// Set the static methods of the class to be exported.
    /// See the [`static_methods!`](crate::static_methods) macro for a convenient way to define methods.
    pub const fn static_methods(mut self, static_methods: &'static [GlobalMetadata]) -> Self {
        self.static_methods = static_methods;
        self
    }

    /// Build the final [`ClassExport`] instance.
    pub const fn build(self) -> ClassExport<C> {
        ClassExport {
            base: self.base,
            methods: self.methods,
            static_methods: self.static_methods,
        }
    }
}

/// A single struct export.
/// This can be used to define a custom struct to be exported to the game.
#[derive(Debug)]
pub struct StructExport<C> {
    base: Option<&'static str>,
    static_methods: &'static [GlobalMetadata],
    _phantom: PhantomData<fn() -> C>,
}

impl<C> StructExport<C> {
    pub fn builder() -> StructExportBuilder<C> {
        StructExportBuilder {
            base: None,
            static_methods: &[],
            _phantom: PhantomData,
        }
    }
}

#[sealed]
impl<C: Default + Clone + NativeRepr> Exportable for StructExport<C> {
    fn register(&self) {
        let mut rtti = RttiSystemMut::get();
        let name_cstr = CString::new(C::NAME).expect("name should be valid");
        let base = self
            .base
            .map(|base| &*rtti.get_class(CName::new(base)).expect("base should exist"));
        let handle = NativeClass::<C>::new_handle(&name_cstr, base);
        rtti.register_class(handle);
    }

    fn post_register(&self) {
        let converted_static_methods = {
            let rtti_ro = RttiSystem::get();
            let class = rtti_ro
                .get_class(CName::new(C::NAME))
                .expect("class should exist");
            self.static_methods
                .iter()
                .map(|m| m.to_rtti_static_method(class))
                .collect::<Vec<_>>()
        };

        let mut rtti_rw = RttiSystemMut::get();
        let class = rtti_rw
            .get_class(CName::new(C::NAME))
            .expect("class should exist");

        for static_method in converted_static_methods {
            class.add_static_method(static_method);
        }
    }
}

/// A builder for [`StructExport`].
#[derive(Debug)]
pub struct StructExportBuilder<C> {
    base: Option<&'static str>,
    static_methods: &'static [GlobalMetadata],
    _phantom: PhantomData<fn() -> C>,
}

impl<C> StructExportBuilder<C> {
    /// Set the base type of the struct to be exported.
    /// Structs do not have a base type by default.
    pub const fn base(mut self, base: &'static str) -> Self {
        self.base = Some(base);
        self
    }

    /// Set the static methods of the struct to be exported.
    /// See the [`static_methods!`](crate::static_methods) macro for a convenient way to define methods.
    pub const fn static_methods(mut self, static_methods: &'static [GlobalMetadata]) -> Self {
        self.static_methods = static_methods;
        self
    }

    /// Build the final [`StructExport`] instance.
    pub const fn build(self) -> StructExport<C> {
        StructExport {
            base: self.base,
            static_methods: self.static_methods,
            _phantom: PhantomData,
        }
    }
}

/// A single global function export.
#[derive(Debug)]
pub struct GlobalExport(pub GlobalMetadata);

#[sealed]
impl Exportable for GlobalExport {
    #[inline]
    fn register(&self) {}

    fn post_register(&self) {
        let converted = self.0.to_rtti();

        let mut rtti = RttiSystemMut::get();
        rtti.register_function(converted);
    }
}

/// Creates a list of exports to be registered within the game's RTTI system.
///
/// # Example
/// ```rust
/// use std::cell::Cell;
///
/// use red4ext_rs::{ClassExport, Exportable, GlobalExport, ScriptClass, class_kind, exports, methods, global};
/// use red4ext_rs::types::IScriptable;
///
/// fn exports() -> impl Exportable {
///     exports![
///         GlobalExport(global!(c"GlobalExample", global_example)),
///         ClassExport::<MyClass>::builder()
///            .base("IScriptable")
///            .methods(methods![
///                c"Value" => MyClass::value,
///                c"SetValue" => MyClass::set_value,
///            ])
///            .build(),
///     ]
/// }
///
/// fn global_example() -> String {
///   "Hello, world!".to_string()
/// }
///
/// #[derive(Debug, Default, Clone)]
/// #[repr(C)]
/// struct MyClass {
///     // You must include the base native class in your Rust struct.
///     base: IScriptable,
///     value: Cell<i32>,
/// }
///
/// impl MyClass {
///    fn value(&self) -> i32 {
///       self.value.get()
///    }
///
///    fn set_value(&self, value: i32) {
///       self.value.set(value)
///    }
/// }
///
/// unsafe impl ScriptClass for MyClass {
///    const NAME: &'static str = "MyClass";
///    type Kind = class_kind::Native;
/// }
#[macro_export]
macro_rules! exports {
    [$export:expr, $($tt:tt)*] => {
        $crate::ExportList::new($export, exports!($($tt)*))
    };
    [$export:expr] => {
        $crate::ExportList::new($export, $crate::ExportNil)
    };
    [] => { $crate::ExportNil }
}

/// Define a list of methods to register with the game. Usually used in conjuction with
/// [`exports!`].
#[macro_export]
macro_rules! methods {
    [$( $($mod:ident)* $name:literal => $ty:ident::$id:ident),*$(,)?] => {
        const { &[$($crate::method!($($mod)* $name, $ty::$id)),*] }
    };
}

/// Define a list of static methods to register with the game. Usually used in conjuction with
/// [`exports!`].
#[macro_export]
macro_rules! static_methods {
    [$( $($mod:ident)* $name:literal => $ty:ident::$id:ident),*$(,)?] => {
        const { &[$($crate::global!($($mod)* $name, $ty::$id)),*] }
    };
}
