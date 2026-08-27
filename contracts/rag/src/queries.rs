#![allow(dead_code)]

use soroban_sdk::{
    contracterror, contracttype, symbol_short, Address, Env, Symbol,
};

/// Represents the lifecycle state of a RAG retrieval request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum RetrievalRequestState {
    Pending = 0,
    Completed = 1,
    Expired = 2,
    Cancelled = 3,
    Rejected = 4,
}

/// Represents a persisted RAG retrieval request.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct RetrievalRequest {
    pub request_id: u64,
    pub requester: Address,
    pub state: RetrievalRequestState,
}

/// Errors that can occur during retrieval request lifecycle operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracterror]
#[repr(u32)]
pub enum RetrievalRequestError {
    RequestNotFound = 1,
    InvalidTransition = 2,
}

/// Storage key used for retrieval requests.
#[derive(Clone)]
#[contracttype]
pub enum RetrievalRequestKey {
    Request(u64),
}

/// Event emitted whenever a retrieval request changes state.
#[derive(Clone)]
#[contracttype]
pub struct RetrievalRequestStateChanged {
    pub request_id: u64,
    pub previous_state: RetrievalRequestState,
    pub new_state: RetrievalRequestState,
}

/// Creates a new retrieval request in the Pending state.
pub fn create_request(
    env: &Env,
    request_id: u64,
    requester: Address,
) -> Result<RetrievalRequest, RetrievalRequestError> {
    let key = RetrievalRequestKey::Request(request_id);

    if env.storage().persistent().has(&key) {
        return Err(RetrievalRequestError::InvalidTransition);
    }

    let request = RetrievalRequest {
        request_id,
        requester,
        state: RetrievalRequestState::Pending,
    };

    env.storage().persistent().set(&key, &request);

    Ok(request)
}

/// Returns the current state of a retrieval request.
pub fn get_state(
    env: &Env,
    request_id: u64,
) -> Result<RetrievalRequestState, RetrievalRequestError> {
    let key = RetrievalRequestKey::Request(request_id);

    let request: RetrievalRequest = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(RetrievalRequestError::RequestNotFound)?;

    Ok(request.state)
}

/// Returns a complete retrieval request.
pub fn get_request(
    env: &Env,
    request_id: u64,
) -> Result<RetrievalRequest, RetrievalRequestError> {
    let key = RetrievalRequestKey::Request(request_id);

    env.storage()
        .persistent()
        .get(&key)
        .ok_or(RetrievalRequestError::RequestNotFound)
}

/// Checks whether a state transition is valid.
pub fn can_transition(
    current: RetrievalRequestState,
    next: RetrievalRequestState,
) -> bool {
    matches!(
        (current, next),
        (
            RetrievalRequestState::Pending,
            RetrievalRequestState::Completed
        ) | (
            RetrievalRequestState::Pending,
            RetrievalRequestState::Expired
        ) | (
            RetrievalRequestState::Pending,
            RetrievalRequestState::Cancelled
        ) | (
            RetrievalRequestState::Pending,
            RetrievalRequestState::Rejected
        )
    )
}

/// Transitions a retrieval request to a new state.
///
/// Only transitions from Pending to one of the terminal states are allowed.
/// Every successful transition emits a state-change event.
pub fn transition_request(
    env: &Env,
    request_id: u64,
    next_state: RetrievalRequestState,
) -> Result<RetrievalRequest, RetrievalRequestError> {
    let key = RetrievalRequestKey::Request(request_id);

    let mut request: RetrievalRequest = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(RetrievalRequestError::RequestNotFound)?;

    let previous_state = request.state;

    if !can_transition(previous_state, next_state) {
        return Err(RetrievalRequestError::InvalidTransition);
    }

    request.state = next_state;

    env.storage().persistent().set(&key, &request);

    let event = RetrievalRequestStateChanged {
        request_id,
        previous_state,
        new_state: next_state,
    };

    env.events().publish(
        (symbol_short!("request"), request_id),
        event,
    );

    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, Address) {
        let env = Env::default();
        let requester = Address::generate(&env);

        (env, requester)
    }

    #[test]
    fn test_create_request_starts_as_pending() {
        let (env, requester) = setup();

        let request =
            create_request(&env, 1, requester.clone()).unwrap();

        assert_eq!(request.request_id, 1);
        assert_eq!(request.requester, requester);
        assert_eq!(
            request.state,
            RetrievalRequestState::Pending
        );

        assert_eq!(
            get_state(&env, 1).unwrap(),
            RetrievalRequestState::Pending
        );
    }

    #[test]
    fn test_pending_can_transition_to_completed() {
        let (env, requester) = setup();

        create_request(&env, 1, requester).unwrap();

        let request = transition_request(
            &env,
            1,
            RetrievalRequestState::Completed,
        )
        .unwrap();

        assert_eq!(
            request.state,
            RetrievalRequestState::Completed
        );

        assert_eq!(
            get_state(&env, 1).unwrap(),
            RetrievalRequestState::Completed
        );
    }

    #[test]
    fn test_pending_can_transition_to_expired() {
        let (env, requester) = setup();

        create_request(&env, 1, requester).unwrap();

        let request = transition_request(
            &env,
            1,
            RetrievalRequestState::Expired,
        )
        .unwrap();

        assert_eq!(
            request.state,
            RetrievalRequestState::Expired
        );
    }

    #[test]
    fn test_pending_can_transition_to_cancelled() {
        let (env, requester) = setup();

        create_request(&env, 1, requester).unwrap();

        let request = transition_request(
            &env,
            1,
            RetrievalRequestState::Cancelled,
        )
        .unwrap();

        assert_eq!(
            request.state,
            RetrievalRequestState::Cancelled
        );
    }

    #[test]
    fn test_pending_can_transition_to_rejected() {
        let (env, requester) = setup();

        create_request(&env, 1, requester).unwrap();

        let request = transition_request(
            &env,
            1,
            RetrievalRequestState::Rejected,
        )
        .unwrap();

        assert_eq!(
            request.state,
            RetrievalRequestState::Rejected
        );
    }

    #[test]
    fn test_completed_cannot_transition() {
        let (env, requester) = setup();

        create_request(&env, 1, requester).unwrap();

        transition_request(
            &env,
            1,
            RetrievalRequestState::Completed,
        )
        .unwrap();

        let result = transition_request(
            &env,
            1,
            RetrievalRequestState::Expired,
        );

        assert_eq!(
            result,
            Err(RetrievalRequestError::InvalidTransition)
        );
    }

    #[test]
    fn test_expired_cannot_transition() {
        let (env, requester) = setup();

        create_request(&env, 1, requester).unwrap();

        transition_request(
            &env,
            1,
            RetrievalRequestState::Expired,
        )
        .unwrap();

        let result = transition_request(
            &env,
            1,
            RetrievalRequestState::Completed,
        );

        assert_eq!(
            result,
            Err(RetrievalRequestError::InvalidTransition)
        );
    }

    #[test]
    fn test_cancelled_cannot_transition() {
        let (env, requester) = setup();

        create_request(&env, 1, requester).unwrap();

        transition_request(
            &env,
            1,
            RetrievalRequestState::Cancelled,
        )
        .unwrap();

        let result = transition_request(
            &env,
            1,
            RetrievalRequestState::Completed,
        );

        assert_eq!(
            result,
            Err(RetrievalRequestError::InvalidTransition)
        );
    }

    #[test]
    fn test_rejected_cannot_transition() {
        let (env, requester) = setup();

        create_request(&env, 1, requester).unwrap();

        transition_request(
            &env,
            1,
            RetrievalRequestState::Rejected,
        )
        .unwrap();

        let result = transition_request(
            &env,
            1,
            RetrievalRequestState::Completed,
        );

        assert_eq!(
            result,
            Err(RetrievalRequestError::InvalidTransition)
        );
    }

    #[test]
    fn test_invalid_pending_to_pending_transition() {
        let (env, requester) = setup();

        create_request(&env, 1, requester).unwrap();

        let result = transition_request(
            &env,
            1,
            RetrievalRequestState::Pending,
        );

        assert_eq!(
            result,
            Err(RetrievalRequestError::InvalidTransition)
        );
    }

    #[test]
    fn test_request_not_found() {
        let (env, _) = setup();

        let result = get_state(&env, 999);

        assert_eq!(
            result,
            Err(RetrievalRequestError::RequestNotFound)
        );
    }

    #[test]
    fn test_can_transition() {
        assert!(can_transition(
            RetrievalRequestState::Pending,
            RetrievalRequestState::Completed
        ));

        assert!(can_transition(
            RetrievalRequestState::Pending,
            RetrievalRequestState::Expired
        ));

        assert!(can_transition(
            RetrievalRequestState::Pending,
            RetrievalRequestState::Cancelled
        ));

        assert!(can_transition(
            RetrievalRequestState::Pending,
            RetrievalRequestState::Rejected
        ));

        assert!(!can_transition(
            RetrievalRequestState::Pending,
            RetrievalRequestState::Pending
        ));

        assert!(!can_transition(
            RetrievalRequestState::Completed,
            RetrievalRequestState::Pending
        ));

        assert!(!can_transition(
            RetrievalRequestState::Expired,
            RetrievalRequestState::Completed
        ));
    }
}