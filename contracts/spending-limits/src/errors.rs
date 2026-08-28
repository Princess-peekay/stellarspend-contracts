use soroban_sdk::contracterror;

/// Errors returned by budget-management operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BudgetError {
    /// The spend would exceed the budget's configured cap.
    BudgetExceeded = 1,
    /// No budget exists for the requested identifier.
    BudgetNotFound = 2,
    /// The caller is not authorized to perform this operation.
    Unauthorized = 3,
    /// An attempt was made to pause a budget that is already paused.
    BudgetAlreadyPaused = 4,
    /// An attempt was made to activate a budget that is already active.
    BudgetAlreadyActive = 5,
}
