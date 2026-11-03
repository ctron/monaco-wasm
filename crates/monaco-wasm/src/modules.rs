//! Explicit editor dependencies with a page-long registration lifetime.
use crate::{EditorError, sys};
use std::sync::Arc;
use wasm_bindgen::JsValue;

/// An extension of the shared Monaco runtime.
///
/// Editors initialize modules in list order before creating their model.
/// Successful IDs initialize once per page, even across editor disposal and
/// remounts. Registration is visible to all editors and is never undone.
/// The first successful registration for an ID wins, including its configuration.
/// Implementations must use a unique, stable ID and clean up partial work
/// before returning an error; failed initialization may be retried. Worker
/// factories registered through the context are rolled back automatically.
pub trait Module: std::fmt::Debug + Send + Sync {
    fn id(&self) -> &'static str;
    fn initialize(&self, context: &ModuleContext) -> Result<(), EditorError>;
}

/// Modules required by an editor, read only when it mounts.
///
/// Convert a single module, array, or vector with `Modules::from(value)` or
/// `value.into()`. Borrowed slices and arrays clone their elements, so those
/// conversions require `Clone`. Iterators can be collected into `Modules`.
/// Clones share ownership of the module instances.
#[derive(Clone, Debug, Default)]
pub struct Modules(Vec<Arc<dyn Module>>);

impl Modules {
    /// Builds a collection of shared modules, including different concrete types.
    pub fn new(modules: Vec<Arc<dyn Module>>) -> Self {
        Self(modules)
    }

    pub(crate) fn initialize(&self) -> Result<(), EditorError> {
        for module in &self.0 {
            if !sys::module_registered(module.id()).map_err(EditorError::js("check module"))? {
                let context = ModuleContext {
                    workers: std::cell::RefCell::new(Vec::new()),
                };
                if let Err(error) = module.initialize(&context) {
                    for label in context.workers.borrow().iter() {
                        let _ = sys::unregister_module_worker(label);
                    }
                    return Err(error);
                }
                sys::mark_module_registered(module.id())
                    .map_err(EditorError::js("register module"))?;
            }
        }
        Ok(())
    }
}

impl<M: Module + 'static> FromIterator<M> for Modules {
    fn from_iter<T: IntoIterator<Item = M>>(modules: T) -> Self {
        Self(
            modules
                .into_iter()
                .map(|module| Arc::new(module) as Arc<dyn Module>)
                .collect(),
        )
    }
}

impl<M: Module + 'static> From<M> for Modules {
    fn from(module: M) -> Self {
        std::iter::once(module).collect()
    }
}

impl<M: Module + 'static> From<Vec<M>> for Modules {
    fn from(modules: Vec<M>) -> Self {
        modules.into_iter().collect()
    }
}

impl<M: Module + 'static, const N: usize> From<[M; N]> for Modules {
    fn from(modules: [M; N]) -> Self {
        modules.into_iter().collect()
    }
}

impl<M: Module + Clone + 'static> From<&[M]> for Modules {
    fn from(modules: &[M]) -> Self {
        modules.iter().cloned().collect()
    }
}

impl<M: Module + Clone + 'static, const N: usize> From<&[M; N]> for Modules {
    fn from(modules: &[M; N]) -> Self {
        Self::from(modules.as_slice())
    }
}

impl Eq for Modules {}

impl PartialEq for Modules {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.0.iter().zip(&other.0).all(|(a, b)| a.id() == b.id())
    }
}

/// Access to the page's existing Monaco instance and worker registry.
/// Available only during module initialization.
pub struct ModuleContext {
    workers: std::cell::RefCell<Vec<String>>,
}

impl ModuleContext {
    pub fn monaco(&self) -> Result<JsValue, EditorError> {
        sys::module_monaco().map_err(EditorError::js("initialize runtime"))
    }

    /// Installs a worker factory. Existing labels cannot be replaced.
    pub fn register_worker(
        &self,
        label: &str,
        factory: &js_sys::Function,
    ) -> Result<(), EditorError> {
        sys::register_module_worker(label, factory).map_err(EditorError::js("register worker"))?;
        self.workers.borrow_mut().push(label.to_owned());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone, Debug)]
    struct TestModule(&'static str);
    impl Module for TestModule {
        fn id(&self) -> &'static str {
            self.0
        }
        fn initialize(&self, _: &ModuleContext) -> Result<(), EditorError> {
            Ok(())
        }
    }
    #[test]
    fn arrays_own_modules_and_preserve_order_across_clones() {
        let modules: Modules = [TestModule("first"), TestModule("second")].into();
        let cloned = modules.clone();
        drop(modules);
        assert_eq!(
            cloned
                .0
                .iter()
                .map(|module| module.id())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
        assert_eq!(
            cloned,
            Modules::from([TestModule("first"), TestModule("second")])
        );
        assert_eq!(Modules::from([] as [TestModule; 0]), Modules::default());
    }
    #[test]
    fn single_vector_slice_and_iterator_conversions_agree() {
        assert_eq!(
            Modules::from(TestModule("first")),
            Modules::from([TestModule("first")])
        );
        let source = vec![TestModule("first"), TestModule("second")];
        let expected = Modules::from(source.clone());
        let slice = Modules::from(source.as_slice());
        let iterator: Modules = source.into_iter().collect();
        assert_eq!(slice, expected);
        assert_eq!(iterator, expected);
        let array = [TestModule("first"), TestModule("second")];
        assert_eq!(Modules::from(&array), expected);
        assert_eq!(Modules::from(Vec::<TestModule>::new()), Modules::default());
        assert_eq!(Modules::from(&[] as &[TestModule]), Modules::default());
    }
}
