mod tests;
use cosmwasm_std::Addr;
pub const DORA_DEMON: &str = "peaka";

pub fn user1() -> Addr {
    Addr::unchecked("dora19lpkv3pt568wxmeuqewq4kr4qsfjax2vs4rxt4")
}

pub fn user2() -> Addr {
    Addr::unchecked("dora1aj0s6d0lhuyaztm8zrh69p73eh08zvqnh5kz6y")
}

pub fn admin() -> Addr {
    Addr::unchecked("dora1t58t7azqzq26406uwehgnfekal5kzym3m9lz4k")
}

pub fn operator() -> Addr {
    Addr::unchecked("dora1qdagdkg9me4253h9qyvx83sd4gpta6rzh2fa0j")
}

pub fn beneficiary() -> Addr {
    Addr::unchecked("dora1tuu2qpj0ytj2k7fta7u5fwruzeyyfqj5q4vq23")
}
