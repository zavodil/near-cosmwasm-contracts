use crate::error::ContractError;
use crate::*;

#[near_bindgen]
impl Contract {
    #[init]
    pub fn instantiate(
        arbiter: AccountId,
        recipient: AccountId,
        end_height: Option<BlockHeight>,
        end_time: Option<BlockHeight>,
    ) -> Self {
        assert!(
            !is_expired(end_height, end_time),
            "{}",
            ContractError::Expired {
                end_height,
                end_time
            }
        );

        Self {
            arbiter,
            recipient,
            source: env::predecessor_account_id(),
            end_height,
            end_time,
        }
    }

    pub fn try_approve(&self, quantity: Option<Balance>) -> PromiseOrValue<ContractError> {
        if env::predecessor_account_id() != self.arbiter {
            return PromiseOrValue::Value(ContractError::Unauthorized);
        };

        // throws error if state is expired
        if is_expired(self.end_height, self.end_time) {
            return PromiseOrValue::Value(ContractError::Expired {
                end_height: self.end_height,
                end_time: self.end_time,
            });
        }

        let amount = if let Some(quantity) = quantity {
            quantity
        } else {
            // release everything
            env::account_balance()
        };

        PromiseOrValue::Promise(send_tokens(self.recipient.clone(), amount))
    }

    pub fn try_refund(&self) -> PromiseOrValue<ContractError> {
        // anyone can try to refund, as long as the contract is expired
        if !is_expired(self.end_height, self.end_time) {
            return PromiseOrValue::Value(ContractError::NotExpired);
        }

        let balance = env::account_balance();
        PromiseOrValue::Promise(send_tokens(self.source.clone(), balance))
    }

    pub fn query_arbiter(&self) -> AccountId {
        self.arbiter.clone()
    }
}

// this is a helper to move the tokens, so the business logic is easy to read
fn send_tokens(to_address: AccountId, amount: Balance) -> Promise {
    Promise::new(to_address).transfer(amount)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, ONE_NEAR};

    use super::*;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn init_expire_by_height(height: u64) -> Contract {
        Contract::instantiate(
            AccountId::new_unchecked("verifies".to_string()),
            AccountId::new_unchecked("benefits".to_string()),
            Some(height),
            None,
        )
    }

    #[test]
    fn proper_initialization() {
        init_expire_by_height(1000);

        let context = get_context(accounts(1));
        testing_env!(context.build());

        Contract::instantiate(
            AccountId::new_unchecked("verifies".to_string()),
            AccountId::new_unchecked("benefits".to_string()),
            Some(1000),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "Escrow expired (end_height 1000 end_time 0)")]
    fn cannot_initialize_expired() {
        let mut context = get_context(accounts(1));
        context.block_index(1001);
        testing_env!(context.build());

        init_expire_by_height(1000);
    }

    #[test]
    fn init_and_query() {
        let state = init_expire_by_height(1000);
        assert_eq!(state.query_arbiter().to_string(), "verifies");
    }

    #[test]
    fn execute_approve() {
        let initial_balance = env::account_balance();
        let mut context = get_context(accounts(1));

        testing_env!(context.attached_deposit(1000 * ONE_NEAR).build());
        let state = init_expire_by_height(1000);

        // balance changed in init
        testing_env!(context
            .predecessor_account_id(AccountId::new_unchecked("creator".to_string()))
            .block_index(876)
            .build());

        assert_eq!(env::account_balance(), initial_balance + 1000 * ONE_NEAR);

        // beneficiary cannot release it
        match state.try_approve(None) {
            PromiseOrValue::Value(value) => assert_eq!(value, ContractError::Unauthorized),
            PromiseOrValue::Promise(_) => panic!("unexpected error"),
        }

        // verifier cannot release it when expired
        testing_env!(context
            .predecessor_account_id(AccountId::new_unchecked("verifies".to_string()))
            .block_index(1100)
            .build());

        match state.try_approve(None) {
            PromiseOrValue::Value(value) => assert_eq!(
                value,
                ContractError::Expired {
                    end_height: Some(1000),
                    end_time: None,
                }
            ),
            PromiseOrValue::Promise(_) => panic!("unexpected error"),
        }

        // partial release by verifier, before expiration
        testing_env!(context
            .predecessor_account_id(AccountId::new_unchecked("verifies".to_string()))
            .block_index(999)
            .build());

        match state.try_approve(Some(400 * ONE_NEAR)) {
            PromiseOrValue::Value(e) => panic!("unexpected error {}", e),
            PromiseOrValue::Promise(..) => {}
        }

        assert_eq!(env::account_balance(), initial_balance + 600 * ONE_NEAR);

        // complete release by verifier, before expiration
        match state.try_approve(None) {
            PromiseOrValue::Value(e) => panic!("unexpected error {}", e),
            PromiseOrValue::Promise(..) => {}
        }

        assert_eq!(env::account_balance(), 0);
    }

    #[test]
    fn handle_refund() {
        let initial_balance = env::account_balance();
        let mut context = get_context(accounts(1));

        testing_env!(context.attached_deposit(1000 * ONE_NEAR).build());
        let state = init_expire_by_height(1000);

        // balance changed in init
        testing_env!(context
            .predecessor_account_id(AccountId::new_unchecked("creator".to_string()))
            .block_index(876)
            .build());

        assert_eq!(env::account_balance(), initial_balance + 1000 * ONE_NEAR);

        // cannot release when unexpired (height < end_height)
        testing_env!(context
            .predecessor_account_id(AccountId::new_unchecked("anybody".to_string()))
            .block_index(800)
            .build());

        match state.try_refund() {
            PromiseOrValue::Value(value) => assert_eq!(value, ContractError::NotExpired),
            PromiseOrValue::Promise(_) => panic!("unexpected error"),
        }

        // cannot release when unexpired (height == end_height)
        testing_env!(context
            .predecessor_account_id(AccountId::new_unchecked("anybody".to_string()))
            .block_index(1000)
            .build());

        match state.try_refund() {
            PromiseOrValue::Value(value) => assert_eq!(value, ContractError::NotExpired),
            PromiseOrValue::Promise(_) => panic!("unexpected error"),
        }

        // anyone can release after expiration
        testing_env!(context
            .predecessor_account_id(AccountId::new_unchecked("anybody".to_string()))
            .block_index(1001)
            .build());

        match state.try_refund() {
            PromiseOrValue::Value(e) => panic!("unexpected error {}", e),
            PromiseOrValue::Promise(..) => {}
        }

        assert_eq!(env::account_balance(), 0);
    }
}
