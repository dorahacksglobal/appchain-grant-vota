//! Quadratic Grant Contract
//! Author: noodles@dorahacks.com
//! Version: 0.1.0
//! License: Apache-2.0

use cosmwasm_std::{
    coins, Addr, BankMsg, Deps, DepsMut, Empty, Env, Event, MessageInfo, Order, Response, StdError,
    StdResult, Uint128,
};
use cw_storage_plus::{Item, Map};
use std::collections::HashMap;
use sylvia::contract;

#[cfg(not(feature = "library"))]
use sylvia::entry_points;

use crate::{error::ContractError, responses::AdminListResp};

pub struct AGContract<'a> {
    pub(crate) admins: Map<'a, &'a Addr, Empty>,
    pub(crate) beneficiary: Item<'a, Addr>,
    pub(crate) round_id: Item<'a, u64>,
    pub(crate) projects: Map<'a, (&'a str, &'a str), HashMap<String, u128>>, // (round_id, project_id) -> {'arch':100, 'atom':20}
    pub(crate) voters: Map<'a, (&'a str, &'a str, &'a Addr), HashMap<String, u128>>, // (round_id, project_id, voter) -> {'arch':100, 'atom':20}
}

#[cfg_attr(not(feature = "library"), entry_points)]
#[contract]
#[error(ContractError)]
impl AGContract<'_> {
    pub const fn new() -> Self {
        Self {
            admins: Map::new("admins"),
            beneficiary: Item::new("beneficiary"),
            round_id: Item::new("round"),
            projects: Map::new("projects"),
            voters: Map::new("votes"),
        }
    }

    #[msg(instantiate)]
    pub fn instantiate(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        admins: Vec<String>,
    ) -> Result<Response, ContractError> {
        let (deps, _, info) = ctx;

        for admin in admins {
            let admin = deps.api.addr_validate(&admin)?;
            self.admins.save(deps.storage, &admin, &Empty {})?;
        }
        self.round_id.save(deps.storage, &1)?;
        self.beneficiary.save(deps.storage, &info.sender)?;
        Ok(Response::new())
    }

    // ============= Query ============= //
    #[msg(query)]
    pub fn admin_list(&self, ctx: (Deps, Env)) -> StdResult<AdminListResp> {
        let (deps, _) = ctx;

        let admins: Result<_, _> = self
            .admins
            .keys(deps.storage, None, None, Order::Ascending)
            .map(|addr| addr.map(String::from))
            .collect();

        Ok(AdminListResp { admins: admins? })
    }

    #[msg(query)]
    pub fn round_id(&self, ctx: (Deps, Env)) -> StdResult<u64> {
        let (deps, _) = ctx;

        let round_id = self.round_id.may_load(deps.storage)?;
        match round_id {
            Some(round_id) => Ok(round_id),
            None => Err(StdError::generic_err("Round not found")),
        }
    }

    #[msg(query)]
    pub fn project(&self, ctx: (Deps, Env), project_id: u64, round_id: u64) -> StdResult<HashMap<String, u128>> {
        let (deps, _) = ctx;
        let project = self.projects.may_load(
            deps.storage,
            (&round_id.to_string(), &project_id.to_string()),
        )?;

        match project {
            Some(project) => Ok(project),
            None => Err(StdError::generic_err("Project not found")),
        }
    }

    #[msg(query)]
    pub fn project_voter(
        &self,
        ctx: (Deps, Env),
        round_id: u64,
        project_id: u64,
        voter: String,
    ) -> StdResult<HashMap<String, u128>> {
        let (deps, _) = ctx;
        let voter_addr = deps.api.addr_validate(&voter)?;
        let voter = self.voters.may_load(
            deps.storage,
            (&round_id.to_string(), &project_id.to_string(), &voter_addr),
        )?;

        match voter {
            Some(voter) => Ok(voter),
            None => Err(StdError::generic_err("Voter not found")),
        }
    }

    // ============= Execute ============= //
    #[msg(exec)]
    pub fn add_member(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        admin: String,
    ) -> Result<Response, ContractError> {
        let (deps, _, info) = ctx;
        if !self.admins.has(deps.storage, &info.sender) {
            return Err(ContractError::Unauthorized {
                sender: info.sender,
            });
        }

        let admin = deps.api.addr_validate(&admin)?;
        if self.admins.has(deps.storage, &admin) {
            return Err(ContractError::NoDupAddress { address: admin });
        }

        self.admins.save(deps.storage, &admin, &Empty {})?;

        let resp = Response::new()
            .add_attribute("action", "add_member")
            .add_event(Event::new("admin_added").add_attribute("addr", admin));
        Ok(resp)
    }

    #[msg(exec)]
    pub fn set_beneficiary(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        address: String,
    ) -> Result<Response, ContractError> {
        let (deps, _, info) = ctx;
        if !self.admins.has(deps.storage, &info.sender) {
            return Err(ContractError::Unauthorized {
                sender: info.sender,
            });
        }

        let beneficiary = deps.api.addr_validate(&address)?;

        self.beneficiary.save(deps.storage, &beneficiary)?;

        let resp = Response::new()
            .add_attribute("action", "set_beneficiary")
            .add_event(Event::new("set_beneficiary").add_attribute("addr", address));
        Ok(resp)
    }

    #[msg(exec)]
    pub fn batch_vote(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        project_ids: Vec<u64>,
        amounts: Vec<Uint128>,
    ) -> Result<Response, ContractError> {
        let (deps, _, info) = ctx;
        let coin = cw_utils::one_coin(&info)?;
        let denom = coin.denom;

        let round_id = self.round_id.load(deps.storage)?;
        let mut total_amounts: u128 = 0;

        for (project_id, vote) in project_ids.iter().zip(amounts.iter()) {
            let amount = vote.u128();
            total_amounts += amount;

            // Update round project
            let mut project = match self.projects.may_load(
                deps.storage,
                (&round_id.to_string(), &project_id.to_string()),
            )? {
                Some(project) => project,
                None => HashMap::new(),
            };

            if let Some(existing_amount) = project.get(&denom) {
                let new_amount = existing_amount + amount;
                project.insert(denom.clone(), new_amount);
            } else {
                project.insert(denom.clone(), amount);
            }

            self.projects.save(
                deps.storage,
                (&round_id.to_string(), &project_id.to_string()),
                &project,
            )?;

            // Update round project voter
            let mut voter = match self.voters.may_load(
                deps.storage,
                (&round_id.to_string(), &project_id.to_string(), &info.sender),
            )? {
                Some(voter) => voter,
                None => HashMap::new(),
            };

            if let Some(existing_amount) = voter.get(&denom) {
                let new_amount = existing_amount + amount;
                voter.insert(denom.clone(), new_amount);
            } else {
                voter.insert(denom.clone(), amount);
            }

            self.voters.save(
                deps.storage,
                (&round_id.to_string(), &project_id.to_string(), &info.sender),
                &voter,
            )?;
        }
        let transfer = coin.amount.u128();
        if transfer != total_amounts {
            return Err(ContractError::InvalidAmount {
                expected: total_amounts,
                actual: transfer,
            });
        }

        let beneficiary = self.beneficiary.load(deps.storage)?;

        let send_message = BankMsg::Send {
            to_address: beneficiary.to_string(),
            amount: coins(transfer, &denom),
        };

        let resp = Response::new()
            .add_message(send_message)
            .add_attribute("action", "batch_vote")
            .add_event(
                Event::new("batch_vote")
                    .add_attribute("sender", info.sender)
                    .add_attribute("round_id", round_id.to_string())
                    .add_attribute(
                        "projects",
                        format!(
                            "{:?}",
                            project_ids
                                .iter()
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>()
                                .join(",")
                        ),
                    )
                    .add_attribute(
                        "amounts",
                        format!(
                            "{:?}",
                            amounts
                                .iter()
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>()
                                .join(",")
                        ),
                    )
                    .add_attribute("denom", &denom),
            );
        Ok(resp)
    }

    #[msg(exec)]
    pub fn end_round(&self, ctx: (DepsMut, Env, MessageInfo)) -> Result<Response, ContractError> {
        let (deps, _, info) = ctx;
        if !self.admins.has(deps.storage, &info.sender) {
            return Err(ContractError::Unauthorized {
                sender: info.sender,
            });
        }

        let round_id = self.round_id.load(deps.storage)?;
        self.round_id.save(deps.storage, &(round_id + 1))?;

        let resp = Response::new()
            .add_attribute("action", "end_round")
            .add_event(Event::new("end_round").add_attribute("round_id", round_id.to_string()));
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use crate::contract::entry_points::{execute, instantiate, query};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_json, Coin, Uint128};

    use super::*;

    #[test]
    fn admin_list_query() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("sender", &[]),
            InstantiateMsg {
                admins: vec!["admin1".to_owned(), "admin2".to_owned()],
            },
        )
        .unwrap();

        let msg = QueryMsg::AdminList {};
        let resp = query(deps.as_ref(), env, ContractQueryMsg::AGContract(msg)).unwrap();
        let resp: AdminListResp = from_json(&resp).unwrap();
        assert_eq!(
            resp,
            AdminListResp {
                admins: vec!["admin1".to_owned(), "admin2".to_owned()],
            }
        );
    }

    #[test]
    fn add_member() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("sender", &[]),
            InstantiateMsg {
                admins: vec!["admin1".to_owned(), "admin2".to_owned()],
            },
        )
        .unwrap();

        let info = mock_info("admin1", &[]);
        let msg = ExecMsg::AddMember {
            admin: "admin3".to_owned(),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            info,
            ContractExecMsg::AGContract(msg),
        )
        .unwrap();

        let msg = QueryMsg::AdminList {};
        let resp = query(deps.as_ref(), env, ContractQueryMsg::AGContract(msg)).unwrap();
        let resp: AdminListResp = from_json(&resp).unwrap();
        assert_eq!(
            resp,
            AdminListResp {
                admins: vec![
                    "admin1".to_owned(),
                    "admin2".to_owned(),
                    "admin3".to_owned()
                ],
            }
        );
    }

    #[test]
    fn test_all() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // Instantiate
        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("sender", &[]),
            InstantiateMsg {
                admins: vec!["admin1".to_owned(), "admin2".to_owned()],
            },
        )
        .unwrap();

        // Vote
        let msg = ExecMsg::BatchVote {
            project_ids: vec![1],
            amounts: vec![Uint128::from(160000u128)],
        };
        let info = mock_info(
            "user1",
            &[Coin {
                denom: "inj".to_string(),
                amount: Uint128::from(160000u128),
            }],
        );
        execute(
            deps.as_mut(),
            env.clone(),
            info,
            ContractExecMsg::AGContract(msg),
        )
        .unwrap();

        let msg = ExecMsg::BatchVote {
            project_ids: vec![1],
            amounts: vec![Uint128::from(90000u128)],
        };
        let info = mock_info(
            "user1",
            &[Coin {
                denom: "inj".to_string(),
                amount: Uint128::from(90000u128),
            }],
        );
        execute(
            deps.as_mut(),
            env.clone(),
            info,
            ContractExecMsg::AGContract(msg),
        )
        .unwrap();

        let msg = ExecMsg::BatchVote {
            project_ids: vec![2],
            amounts: vec![Uint128::from(160000u128)],
        };
        let info = mock_info(
            "user1",
            &[Coin {
                denom: "inj".to_string(),
                amount: Uint128::from(160000u128),
            }],
        );
        execute(
            deps.as_mut(),
            env.clone(),
            info,
            ContractExecMsg::AGContract(msg),
        )
        .unwrap();

        // End round
        let info = mock_info("admin1", &[]);
        let msg = ExecMsg::EndRound {};
        execute(
            deps.as_mut(),
            env.clone(),
            info,
            ContractExecMsg::AGContract(msg),
        )
        .unwrap();
    }
}
