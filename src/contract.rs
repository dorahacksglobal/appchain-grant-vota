//! Quadratic Grant Contract
//! Author: noodles@dorahacks.com
//! Version: 0.1.0
//! License: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coins, Addr, BankMsg, Deps, DepsMut, Env, Event, MessageInfo, Response, StdError, StdResult,
    Uint128,
};

use cw_storage_plus::{Item, Map};
use std::collections::HashMap;
use sylvia::contract;

#[cfg(not(feature = "library"))]
use sylvia::entry_points;

use crate::error::ContractError;

#[cw_serde]
pub struct Admin {
    pub admin: Addr,
}

impl Admin {
    pub fn is_admin(&self, addr: impl AsRef<str>) -> bool {
        let addr = addr.as_ref();
        self.admin.as_ref() == addr
    }
}

#[cw_serde]
pub struct InstantiateMsgData {
    pub admin: Addr,
    pub operator: Addr,
    pub beneficiary: Addr,
}

pub struct AGContract<'a> {
    pub(crate) admin: Item<'a, Admin>,
    pub(crate) operator: Item<'a, Addr>,
    pub(crate) beneficiary: Item<'a, Addr>,
    pub(crate) projects: Map<'a, &'a str, HashMap<String, u128>>, // project_id -> {'arch':100, 'atom':20}
    pub(crate) voters: Map<'a, (&'a str, &'a Addr), HashMap<String, u128>>, // (project_id, voter) -> {'arch':100, 'atom':20}
}

#[cfg_attr(not(feature = "library"), entry_points)]
#[contract]
#[error(ContractError)]
impl AGContract<'_> {
    pub const fn new() -> Self {
        Self {
            admin: Item::new("admin"),
            operator: Item::new("operator"),
            beneficiary: Item::new("beneficiary"),
            projects: Map::new("projects"),
            voters: Map::new("votes"),
        }
    }

    #[msg(instantiate)]
    pub fn instantiate(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        data: InstantiateMsgData,
    ) -> Result<Response, ContractError> {
        let (deps, _, _) = ctx;
        let InstantiateMsgData {
            admin,
            operator,
            beneficiary,
        } = data;
        let admin = Admin { admin };
        self.admin.save(deps.storage, &admin)?;
        self.operator.save(deps.storage, &operator)?;
        self.beneficiary.save(deps.storage, &beneficiary)?;
        Ok(Response::new())
    }

    // ============= Query ============= //
    #[msg(query)]
    pub fn admin(&self, ctx: (Deps, Env)) -> StdResult<Admin> {
        let (deps, _) = ctx;

        let admin = self.admin.load(deps.storage)?;

        Ok(admin)
    }

    #[msg(query)]
    pub fn operator(&self, ctx: (Deps, Env)) -> StdResult<Addr> {
        let (deps, _) = ctx;

        let operator = self.operator.load(deps.storage)?;

        Ok(operator)
    }

    #[msg(query)]
    pub fn beneficiary(&self, ctx: (Deps, Env)) -> StdResult<Addr> {
        let (deps, _) = ctx;

        let beneficiary = self.beneficiary.load(deps.storage)?;

        Ok(beneficiary)
    }

    #[msg(query)]
    pub fn project(&self, ctx: (Deps, Env), project_id: u64) -> StdResult<HashMap<String, u128>> {
        let (deps, _) = ctx;
        let project = self
            .projects
            .may_load(deps.storage, &project_id.to_string())?;

        match project {
            Some(project) => Ok(project),
            None => Err(StdError::generic_err("Project not found")),
        }
    }

    #[msg(query)]
    pub fn project_voter(
        &self,
        ctx: (Deps, Env),
        project_id: u64,
        voter: String,
    ) -> StdResult<HashMap<String, u128>> {
        let (deps, _) = ctx;
        let voter_addr = deps.api.addr_validate(&voter)?;
        let voter = self
            .voters
            .may_load(deps.storage, (&project_id.to_string(), &voter_addr))?;

        match voter {
            Some(voter) => Ok(voter),
            None => Err(StdError::generic_err("Voter not found")),
        }
    }

    // Only admin can execute
    fn is_admin(&self, deps: Deps, sender: &str) -> StdResult<bool> {
        let admin = self.admin.load(deps.storage)?;
        let can = admin.is_admin(&sender);
        Ok(can)
    }

    // Only operator/admin can execute
    fn is_operator(&self, deps: Deps, sender: &str) -> StdResult<bool> {
        let admin = self.admin.load(deps.storage)?;
        let can_admin = admin.is_admin(&sender);

        let operator = self.operator.load(deps.storage)?;
        let can_operator = sender.to_string() == operator.to_string();

        let can = can_admin || can_operator;
        Ok(can)
    }

    // ============= Execute ============= //
    #[msg(exec)]
    pub fn change_operator(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        address: Addr,
    ) -> Result<Response, ContractError> {
        let (deps, _, info) = ctx;

        if !self.is_admin(deps.as_ref(), info.sender.as_ref())? {
            return Err(ContractError::Unauthorized {
                sender: info.sender,
            });
        }

        self.operator.save(deps.storage, &address)?;

        Ok(Response::new()
            .add_attribute("action", "change_operator")
            .add_attribute("address", address.to_string()))
    }

    #[msg(exec)]
    pub fn set_beneficiary(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        address: String,
    ) -> Result<Response, ContractError> {
        let (deps, _, info) = ctx;

        if !self.is_operator(deps.as_ref(), info.sender.as_ref())? {
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

        let mut total_amounts: u128 = 0;

        for (project_id, vote) in project_ids.iter().zip(amounts.iter()) {
            let amount = vote.u128();
            total_amounts += amount;

            // Update round project
            let mut project = match self
                .projects
                .may_load(deps.storage, &project_id.to_string())?
            {
                Some(project) => project,
                None => HashMap::new(),
            };

            if let Some(existing_amount) = project.get(&denom) {
                let new_amount = existing_amount + amount;
                project.insert(denom.clone(), new_amount);
            } else {
                project.insert(denom.clone(), amount);
            }

            self.projects
                .save(deps.storage, &project_id.to_string(), &project)?;

            // Update round project voter
            let mut voter = match self
                .voters
                .may_load(deps.storage, (&project_id.to_string(), &info.sender))?
            {
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
                (&project_id.to_string(), &info.sender),
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
}

#[cfg(test)]
mod tests {
    use crate::contract::entry_points::{execute, instantiate, query};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_json, Coin, Uint128};

    use super::*;

    #[test]
    fn admin_operator_beneficiary_query() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("sender", &[]),
            InstantiateMsg {
                data: InstantiateMsgData {
                    admin: Addr::unchecked("admin"),
                    operator: Addr::unchecked("operator"),
                    beneficiary: Addr::unchecked("beneficiary"),
                },
            },
        )
        .unwrap();

        let admin_addr: Admin = from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                ContractQueryMsg::AGContract(QueryMsg::Admin {}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            admin_addr,
            Admin {
                admin: Addr::unchecked("admin")
            }
        );

        let operator_addr: Addr = from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                ContractQueryMsg::AGContract(QueryMsg::Operator {}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(operator_addr, Addr::unchecked("operator"));

        let beneficiary_addr: Addr = from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                ContractQueryMsg::AGContract(QueryMsg::Beneficiary {}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(beneficiary_addr, Addr::unchecked("beneficiary"));
    }

    #[test]
    fn change_operator() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("sender", &[]),
            InstantiateMsg {
                data: InstantiateMsgData {
                    admin: Addr::unchecked("admin"),
                    operator: Addr::unchecked("operator"),
                    beneficiary: Addr::unchecked("beneficiary"),
                },
            },
        )
        .unwrap();

        let admin_info = mock_info("admin", &[]);
        let msg = ExecMsg::ChangeOperator {
            address: Addr::unchecked("operator_changed"),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ContractExecMsg::AGContract(msg),
        )
        .unwrap();

        let operator_changed_addr: Addr = from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                ContractQueryMsg::AGContract(QueryMsg::Operator {}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(operator_changed_addr, Addr::unchecked("operator_changed"));

        let no_access_info = mock_info("admin1", &[]);
        let no_access_change_operator_error = execute(
            deps.as_mut(),
            env.clone(),
            no_access_info.clone(),
            ContractExecMsg::AGContract(ExecMsg::ChangeOperator {
                address: Addr::unchecked("operator_changed"),
            }),
        )
        .unwrap_err();

        assert_eq!(
            no_access_change_operator_error,
            ContractError::Unauthorized {
                sender: no_access_info.clone().sender
            },
        );
        let no_access_set_beneficiary_error = execute(
            deps.as_mut(),
            env.clone(),
            no_access_info.clone(),
            ContractExecMsg::AGContract(ExecMsg::SetBeneficiary {
                address: "0x1a4B7626f3627EECf64b7beFFE599625080b28aA".to_string(),
            }),
        )
        .unwrap_err();

        assert_eq!(
            no_access_set_beneficiary_error,
            ContractError::Unauthorized {
                sender: no_access_info.clone().sender
            },
        );

        let inited_beneficiary_addr: Addr = from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                ContractQueryMsg::AGContract(QueryMsg::Beneficiary {}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(inited_beneficiary_addr, Addr::unchecked("beneficiary"));

        execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ContractExecMsg::AGContract(ExecMsg::SetBeneficiary {
                address: "dora1xuentny2lw3zx5rheevpudut0we3ghf3d4dm5p".to_string(),
            }),
        )
        .unwrap();

        let set_beneficiary_addr: Addr = from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                ContractQueryMsg::AGContract(QueryMsg::Beneficiary {}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            set_beneficiary_addr,
            Addr::unchecked("dora1xuentny2lw3zx5rheevpudut0we3ghf3d4dm5p")
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
                data: InstantiateMsgData {
                    admin: Addr::unchecked("admin"),
                    operator: Addr::unchecked("operator"),
                    beneficiary: Addr::unchecked("beneficiary"),
                },
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
    }
}
