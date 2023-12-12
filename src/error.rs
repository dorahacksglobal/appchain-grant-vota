use cosmwasm_std::{Addr, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{sender} is not a contract admin")]
    Unauthorized { sender: Addr },

    #[error("Payment error: {0}")]
    Payment(#[from] PaymentError),

    #[error("{address} is already an admin")]
    NoDupAddress { address: Addr },

    #[error("expected {expected} but got {actual}")]
    InvalidAmount { expected: u128, actual: u128 },
}
