use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("F505: Cyclic dependency detected in variable graph: {cycle}")]
    CyclicDependency { cycle: String },

    #[error("F401: Variable not found: {var}")]
    VariableNotFound { var: String },

    #[error("F405: Invalid variable path (missing field): {path}")]
    InvalidVariablePath { path: String },

    #[error("F453: Runtime input validation failed: {message}")]
    InputValidationFailed { message: String },

    #[error("F451: Type mismatch: {message}")]
    TypeMismatch { message: String },

    #[error("F452: Constraint violation: {message}")]
    ConstraintViolation { message: String },

    #[error("F456: Invalid or missing effect declaration: {message}")]
    InvalidEffectDeclaration { message: String },

    #[error("F454: Policy denied operation: {name}")]
    PolicyDenied { name: String },

    #[error("F455: Guard undecidable for operation: {name}")]
    GuardUndecidable { name: String },

    #[error("F902: Compute gas exhausted (limit: {limit})")]
    GasExhausted { limit: usize },

    #[error("F801: Lens execution failed: {message}")]
    LensExecutionFailed { message: String },

    #[error("F802: Unknown lens: {name}")]
    UnknownLens { name: String },

    #[error("F901: Critical sections exceed budget (budget: {budget}, required: {required})")]
    BudgetExceeded { budget: usize, required: usize },

    #[error("F803: Execution error: {message}")]
    ExecutionError { message: String },
}

pub type EngineResult<T> = Result<T, EngineError>;
