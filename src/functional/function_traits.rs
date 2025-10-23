//! Function Traits for Pure Functional Programming
//!
//! This module defines the core traits and types for pure functions
//! used in the functional programming infrastructure. All functions
//! must be pure (deterministic, side-effect free), thread-safe,
//! and composable.

#[allow(dead_code)]
use std::fmt::Debug;
use std::hash::Hash;

/// Core trait for pure functions that can be registered and composed.
///
/// Pure functions must be:
/// - Deterministic: Same input always produces same output
/// - Side-effect free: No mutations, I/O, or external state changes
/// - Thread-safe: Send + Sync for concurrent execution
/// - Composable: Can be chained with other pure functions
pub trait PureFunction<Input, Output>: Send + Sync + 'static {
    /// Execute the pure function with the given input.
    ///
    /// # Arguments
    /// * `input` - The input value to process
    ///
    /// # Returns
    /// The output value after processing
    fn call(&self, input: Input) -> Output;

    /// Get the function signature for type checking and composition.
    ///
    /// # Returns
    /// A string representation of the function signature
    fn signature(&self) -> &'static str;

    /// Get the function category for organization and lookup.
    ///
    /// # Returns
    /// The category this function belongs to
    fn category(&self) -> FunctionCategory;

    /// Determine whether this function can be composed with another `PureFunction` that accepts this
    /// function's output.
    ///
    /// # Returns
    ///
    /// `true` if composition is allowed, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// # use crate::{FunctionWrapper, FunctionCategory, PureFunction};
    /// let f = FunctionWrapper::new(|x: i32| x + 1, "inc", FunctionCategory::Mathematical);
    /// let g = FunctionWrapper::new(|x: i32| x * 2, "mul", FunctionCategory::Mathematical);
    /// let f_obj: &dyn PureFunction<i32, i32> = &f;
    /// let g_obj: &dyn PureFunction<i32, i32> = &g;
    /// assert!(f_obj.can_compose_with(g_obj));
    /// ```
    fn can_compose_with(&self, _: &dyn PureFunction<Output, Output>) -> bool {
        true
    }
}

/// Function categories for organization and lookup optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionCategory {
    /// Data transformation functions (map, filter, etc.)
    Transformation,
    /// Data aggregation functions (sum, count, etc.)
    Aggregation,
    /// Validation functions
    Validation,
    /// Data filtering functions
    Filtering,
    /// Data sorting and ordering functions
    Sorting,
    /// Data grouping functions
    Grouping,
    /// Mathematical operations
    Mathematical,
    /// String processing functions
    StringProcessing,
    /// Custom business logic functions
    BusinessLogic,
}

/// Generic function wrapper that implements PureFunction trait.
///
/// This allows storing different function types in the registry
/// while maintaining type safety and purity guarantees.
pub struct FunctionWrapper<Input, Output, F>
where
    Input: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    F: Fn(Input) -> Output + Send + Sync + 'static,
{
    function: F,
    signature: &'static str,
    category: FunctionCategory,
    _phantom: std::marker::PhantomData<(Input, Output)>,
}

impl<Input, Output, F> FunctionWrapper<Input, Output, F>
where
    Input: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    F: Fn(Input) -> Output + Send + Sync + 'static,
{
    /// Constructs a FunctionWrapper that holds a pure function together with its signature and category.
    ///
    /// Returns a new `FunctionWrapper` configured with the provided function, signature, and category.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::{FunctionWrapper, FunctionCategory, PureFunction};
    ///
    /// let wrapper = FunctionWrapper::new(|x: i32| x + 1, "inc(i32)->i32", FunctionCategory::Mathematical);
    /// assert_eq!(wrapper.call(1), 2);
    /// ```
    pub fn new(function: F, signature: &'static str, category: FunctionCategory) -> Self {
        Self {
            function,
            signature,
            category,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Input, Output, F> PureFunction<Input, Output> for FunctionWrapper<Input, Output, F>
where
    Input: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    F: Fn(Input) -> Output + Send + Sync + 'static,
{
    /// Invokes the wrapped pure function with the provided input and returns its output.
    ///
    /// # Returns
    ///
    /// The output produced by applying the wrapped function to `input`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// // Construct a simple wrapper for a closure that doubles an i32.
    /// let wrapper = crate::FunctionWrapper::new(|x: i32| x * 2, "double_i32", crate::FunctionCategory::Mathematical);
    /// let result = wrapper.call(3);
    /// assert_eq!(result, 6);
    /// ```
    fn call(&self, input: Input) -> Output {
        (self.function)(input)
    }

    /// Returns the stored function signature string.
    ///
    /// # Returns
    ///
    /// The signature associated with this container (a `'static` string).
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::{FunctionWrapper, FunctionContainer, FunctionCategory};
    ///
    /// let wrapper = FunctionWrapper::new(|x: i32| x + 1, "inc", FunctionCategory::Transformation);
    /// let container = FunctionContainer::new(wrapper, "inc", FunctionCategory::Transformation);
    /// assert_eq!(container.signature(), "inc");
    /// ```
    fn signature(&self) -> &'static str {
        self.signature
    }

    /// Gets the function's category used for organization and lookup.
    ///
    /// # Examples
    ///
    /// ```
    /// let wrapper = FunctionWrapper::new(|x: i32| x + 1, "inc", FunctionCategory::Transformation);
    /// assert_eq!(wrapper.category(), FunctionCategory::Transformation);
    /// ```
    ///
    /// # Returns
    ///
    /// The `FunctionCategory` value representing this function's category.
    fn category(&self) -> FunctionCategory {
        self.category
    }
}

/// Type-erased function container for storage in the registry.
///
/// This allows storing functions with different input/output types
/// in the same collection while maintaining thread safety.
pub struct FunctionContainer {
    /// Type-erased callable function
    callable: Box<dyn Callable>,
    /// Function signature
    signature: &'static str,
    /// Function category
    category: FunctionCategory,
    /// Type information for composition checking
    input_type_id: std::any::TypeId,
    output_type_id: std::any::TypeId,
}

impl FunctionContainer {
    /// Constructs a type-erased container holding a pure function along with its signature and category.
    ///
    /// The `signature` should uniquely identify the function for registration and lookup; `category` is used for organization.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::Any;
    ///
    /// // Create a FunctionWrapper from a closure using the provided macro,
    /// // then wrap it in a FunctionContainer.
    /// let func = pure_closure!("add_one", crate::FunctionCategory::Mathematical, |x: i32| x + 1);
    /// let container = crate::FunctionContainer::new(func, "add_one", crate::FunctionCategory::Mathematical);
    ///
    /// // Call with the correct input type
    /// let input: Box<dyn Any> = Box::new(5i32);
    /// let output = container.try_call(input).expect("type matched");
    /// let result = *output.downcast::<i32>().unwrap();
    /// assert_eq!(result, 6);
    /// ```
    pub fn new<Input, Output, F>(
        function: F,
        signature: &'static str,
        category: FunctionCategory,
    ) -> Self
    where
        Input: Send + Sync + 'static,
        Output: Send + Sync + 'static,
        F: PureFunction<Input, Output> + 'static,
    {
        // Create a closure that calls the PureFunction
        let closure = move |input: Input| function.call(input);
        let wrapper = FunctionWrapper::new(closure, signature, category);
        Self {
            callable: Box::new(wrapper),
            signature,
            category,
            input_type_id: std::any::TypeId::of::<Input>(),
            output_type_id: std::any::TypeId::of::<Output>(),
        }
    }

    /// Return the stored signature for the wrapped function.
    ///
    /// # Examples
    ///
    /// ```
    /// let wrapper = FunctionWrapper::new(|x: i32| x + 1, "inc", FunctionCategory::Transformation);
    /// assert_eq!(wrapper.signature(), "inc");
    /// ```
    pub fn signature(&self) -> &'static str {
        self.signature
    }

    /// Retrieves the function's category used for organization and lookup.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::{FunctionWrapper, FunctionCategory};
    ///
    /// let wrapper = FunctionWrapper::new(|x: i32| x + 1, "inc", FunctionCategory::Transformation);
    /// assert_eq!(wrapper.category(), FunctionCategory::Transformation);
    /// ```
    pub fn category(&self) -> FunctionCategory {
        self.category
    }

    /// The `TypeId` of the container's input type.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::TypeId;
    /// // Construct a wrapper and container for a simple i32 -> i32 function.
    /// let wrapper = crate::FunctionWrapper::new(|x: i32| x + 1, "inc", crate::FunctionCategory::Transformation);
    /// let container = crate::FunctionContainer::new::<i32, i32, _>(wrapper, "inc", crate::FunctionCategory::Transformation);
    /// let id = container.input_type_id();
    /// assert_eq!(id, TypeId::of::<i32>());
    /// ```
    pub fn input_type_id(&self) -> std::any::TypeId {
        self.input_type_id
    }

    /// Returns the stored output TypeId used for runtime composition checks.
    ///
    /// The returned `TypeId` identifies the function's output type.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::TypeId;
    /// // `container` is a FunctionContainer created elsewhere for a function that returns `i32`.
    /// // assert_eq!(container.output_type_id(), TypeId::of::<i32>());
    /// ```
    pub fn output_type_id(&self) -> std::any::TypeId {
        self.output_type_id
    }

    /// Invoke the stored callable with a type-erased input if the input's runtime type matches this container's expected input type.
    ///
    /// # Returns
    ///
    /// `Some(Box<dyn Any>)` containing the boxed output when the input's TypeId equals the container's input type id, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::Any;
    /// use your_crate::function_module::{FunctionContainer, FunctionCategory};
    ///
    /// // Create a container for a function that increments an i32
    /// let container = FunctionContainer::new(|x: i32| x + 1, "inc_i32", FunctionCategory::Mathematical);
    ///
    /// let input: Box<dyn Any> = Box::new(41i32);
    /// let output_any = container.try_call(input).expect("type should match");
    /// let output = output_any.downcast::<i32>().expect("downcast to i32");
    /// assert_eq!(*output, 42);
    /// ```
    pub fn try_call(&self, input: Box<dyn std::any::Any>) -> Option<Box<dyn std::any::Any>> {
        // Check if the input type matches by checking the inner type
        if (*input).type_id() != self.input_type_id {
            return None;
        }

        // Call the function using the Callable trait
        Some(self.callable.call_boxed(input))
    }
}

/// Object-safe callable trait for type-erased function calls
pub trait Callable: Send + Sync {
    fn call_boxed(&self, input: Box<dyn std::any::Any>) -> Box<dyn std::any::Any>;
    fn input_type_id(&self) -> std::any::TypeId;
    fn output_type_id(&self) -> std::any::TypeId;
}

impl<Input, Output, F> Callable for FunctionWrapper<Input, Output, F>
where
    Input: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    F: Fn(Input) -> Output + Send + Sync + 'static,
{
    /// Calls the wrapped function using a type-erased input and returns the result boxed as `Any`.
    ///
    /// Panics if the provided input's concrete type does not match the function's expected input type.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::Any;
    /// use std::any::TypeId;
    /// // Construct a FunctionWrapper directly for the example
    /// let f = |x: i32| x + 1;
    /// let wrapper = crate::FunctionWrapper::new(f, "inc", crate::FunctionCategory::Mathematical);
    /// let boxed_in: Box<dyn Any> = Box::new(41i32);
    /// let boxed_out = wrapper.call_boxed(boxed_in);
    /// assert_eq!(*boxed_out.downcast::<i32>().unwrap(), 42);
    /// ```
    fn call_boxed(&self, input: Box<dyn std::any::Any>) -> Box<dyn std::any::Any> {
        // Check type before downcast
        if (*input).type_id() != std::any::TypeId::of::<Input>() {
            panic!("Type mismatch in callable");
        }

        // Downcast the input to the expected type
        if let Ok(input) = input.downcast::<Input>() {
            let result = (self.function)(*input);
            Box::new(result)
        } else {
            panic!("Type mismatch in callable")
        }
    }

    /// Get the `TypeId` corresponding to the wrapper's input type.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::TypeId;
    /// use your_crate::{FunctionWrapper, FunctionCategory};
    ///
    /// let wrapper = FunctionWrapper::new(|x: i32| x + 1, "inc", FunctionCategory::Transformation);
    /// let id = wrapper.input_type_id();
    /// assert_eq!(id, TypeId::of::<i32>());
    /// ```
    ///
    /// # Returns
    ///
    /// The `TypeId` for `Input`.
    fn input_type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Input>()
    }

    /// Get the TypeId for the function's output type.
    ///
    /// The returned `TypeId` corresponds to the concrete `Output` type parameter of this wrapper.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::TypeId;
    /// // assume FunctionWrapper and FunctionCategory are in scope
    /// let wrapper = FunctionWrapper::new(|x: i32| x * 2, "double_i32", FunctionCategory::Mathematical);
    /// assert_eq!(wrapper.output_type_id(), TypeId::of::<i32>());
    /// ```
    fn output_type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Output>()
    }
}

#[macro_export]
macro_rules! pure_function {
    ($name:ident, $input:ty, $output:ty, $category:expr, $body:expr) => {
        pub struct $name;

        impl $crate::functional::function_traits::PureFunction<$input, $output> for $name {
            fn call(&self, input: $input) -> $output {
                $body(input)
            }

            fn signature(&self) -> &'static str {
                stringify!($name)
            }

            fn category(&self) -> $crate::functional::function_traits::FunctionCategory {
                $category
            }
        }
    };
}

#[macro_export]
macro_rules! pure_closure {
    ($signature:expr, $category:expr, $closure:expr) => {
        $crate::functional::function_traits::FunctionWrapper::new($closure, $signature, $category)
    };
}
