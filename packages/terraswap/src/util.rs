use cosmwasm_std::{DepsMut, StdError, StdResult};
use cw2::{get_contract_version, set_contract_version};

pub fn migrate_version(
    deps: DepsMut,
    target_contract_version: &str,
    name: &str,
    version: &str,
) -> StdResult<()> {
    let prev_version = get_contract_version(deps.as_ref().storage)?;
    if prev_version.contract != name {
        return Err(StdError::generic_err("invalid contract"));
    }

    if prev_version.version != target_contract_version {
        return Err(StdError::generic_err(format!(
            "invalid contract version. target {}, but source is {}",
            target_contract_version, prev_version.version
        )));
    }

    set_contract_version(deps.storage, name, version)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::mock_querier::mock_dependencies;

    use super::*;

    const TARGET_VERSION: &str = "version";
    const NAME: &str = "name";
    const CURRENT_VERSION: &str = "c_version";

    #[test]
    pub fn normal_migration() {
        let mut deps = mock_dependencies(&[]);
        set_contract_version(deps.as_mut().storage, NAME, TARGET_VERSION).unwrap();

        let res = migrate_version(deps.as_mut(), TARGET_VERSION, NAME, CURRENT_VERSION);

        assert_eq!(res, Ok(()));

        let version = get_contract_version(deps.as_ref().storage).unwrap();

        assert_eq!(version.contract, NAME);

        assert_eq!(version.version, CURRENT_VERSION);
    }

    #[test]
    pub fn failed_migration_with_invalid_contract_name() {
        let mut deps = mock_dependencies(&[]);
        set_contract_version(deps.as_mut().storage, NAME, TARGET_VERSION).unwrap();

        let res = migrate_version(
            deps.as_mut(),
            TARGET_VERSION,
            "invalid_name",
            CURRENT_VERSION,
        );

        assert_eq!(res, Err(StdError::generic_err("invalid contract")));

        let version = get_contract_version(deps.as_ref().storage).unwrap();

        assert_eq!(version.contract, NAME);

        assert_eq!(version.version, TARGET_VERSION);
    }

    #[test]
    pub fn failed_migration_with_invalid_target_version() {
        let mut deps = mock_dependencies(&[]);
        set_contract_version(deps.as_mut().storage, NAME, TARGET_VERSION).unwrap();

        let res = migrate_version(deps.as_mut(), "invalide_version", NAME, CURRENT_VERSION);

        assert_eq!(
            res,
            Err(StdError::generic_err(format!(
                "invalid contract version. target {}, but source is {}",
                "invalide_version", TARGET_VERSION
            )))
        );

        let version = get_contract_version(deps.as_ref().storage).unwrap();

        assert_eq!(version.contract, NAME);

        assert_eq!(version.version, TARGET_VERSION);
    }
}
