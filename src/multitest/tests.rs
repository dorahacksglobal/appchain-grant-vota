use std::collections::HashMap;

use crate::contract::multitest_utils::CodeId;
use crate::contract::InstantiateMsgData;
use crate::multitest::*;
use cosmwasm_std::{coin, coins, Coin, Uint128};
use cw_multi_test;
use sylvia::multitest::App;

#[test]
fn mintable() {
    let user_coin_amount = 410000; //  160000 + 90000 +160000

    let baseapp = cw_multi_test::App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &user1(), coins(user_coin_amount, DORA_DEMON))
            .unwrap();
        router
            .bank
            .init_balance(storage, &user2(), coins(user_coin_amount, DORA_DEMON))
            .unwrap();
    });

    let app = App::new(baseapp);

    let code_id = CodeId::store_code(&app);

    let contract = code_id
        .instantiate(InstantiateMsgData {
            admin: admin(),
            operator: operator(),
            beneficiary: beneficiary(),
        })
        .with_label("AEZ contract")
        .call(&user1().to_string())
        .unwrap();

    let user1_balance = app
        .app()
        .wrap()
        .query_balance(user1().to_string(), DORA_DEMON)
        .unwrap();

    assert_eq!(
        user1_balance,
        Coin {
            amount: Uint128::from(user_coin_amount),
            denom: DORA_DEMON.to_string()
        }
    );
    // User1 Vote 160000u128
    contract
        .batch_vote(vec![1], vec![Uint128::from(160000u128)])
        .with_funds(&[coin(160000u128, DORA_DEMON)])
        .call(&user1().to_string())
        .unwrap();

    // User1 Vote 90000u128
    contract
        .batch_vote(vec![1], vec![Uint128::from(90000u128)])
        .with_funds(&[coin(90000u128, DORA_DEMON)])
        .call(&user1().to_string())
        .unwrap();

    // User1 Vote 160000u128
    contract
        .batch_vote(vec![1], vec![Uint128::from(160000u128)])
        .with_funds(&[coin(160000u128, DORA_DEMON)])
        .call(&user1().to_string())
        .unwrap();

    let user1_balance_after_vote = app
        .app()
        .wrap()
        .query_balance(user1().to_string(), DORA_DEMON)
        .unwrap();

    assert_eq!(
        user1_balance_after_vote,
        Coin {
            amount: Uint128::from(0u128),
            denom: DORA_DEMON.to_string()
        }
    );

    // User2 Vote 100000u128
    contract
        .batch_vote(vec![2], vec![Uint128::from(100000u128)])
        .with_funds(&[coin(100000u128, DORA_DEMON)])
        .call(&user2().to_string())
        .unwrap();

    // User2 Vote 200000u128
    contract
        .batch_vote(vec![1], vec![Uint128::from(200000u128)])
        .with_funds(&[coin(200000u128, DORA_DEMON)])
        .call(&user2().to_string())
        .unwrap();

    let user2_balance_after_vote = app
        .app()
        .wrap()
        .query_balance(user2().to_string(), DORA_DEMON)
        .unwrap();

    assert_eq!(
        user2_balance_after_vote,
        Coin {
            amount: Uint128::from(user_coin_amount - 100000u128 - 200000u128),
            denom: DORA_DEMON.to_string()
        }
    );

    let beneficiary_balance_after_vote = app
        .app()
        .wrap()
        .query_balance(beneficiary().to_string(), DORA_DEMON)
        .unwrap();

    assert_eq!(
        beneficiary_balance_after_vote,
        Coin {
            amount: Uint128::from(160000u128 + 90000u128 + 160000u128 + 100000u128 + 200000u128),
            denom: DORA_DEMON.to_string()
        }
    );

    let project1_info = contract.project(1u64).unwrap();
    assert_eq!(
        project1_info,
        HashMap::from([(
            DORA_DEMON.to_string(),
            160000u128 + 90000u128 + 160000u128 + 200000u128
        )])
    );

    let project2_info = contract.project(2u64).unwrap();
    assert_eq!(
        project2_info,
        HashMap::from([(DORA_DEMON.to_string(), 100000u128)])
    );

    let user1_vote_project1_info = contract.project_voter(1u64, user1().to_string()).unwrap();
    assert_eq!(
        user1_vote_project1_info,
        HashMap::from([(DORA_DEMON.to_string(), 160000u128 + 90000u128 + 160000u128)])
    );

    let user2_vote_project2_info = contract.project_voter(2u64, user2().to_string()).unwrap();
    assert_eq!(
        user2_vote_project2_info,
        HashMap::from([(DORA_DEMON.to_string(), 100000u128)])
    );

    let user2_vote_project1_info = contract.project_voter(1u64, user2().to_string()).unwrap();
    assert_eq!(
        user2_vote_project1_info,
        HashMap::from([(DORA_DEMON.to_string(), 200000u128)])
    );
}
