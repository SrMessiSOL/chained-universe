use anchor_lang::prelude::*;

#[error_code]
pub enum GameStateError {
    #[msg("The caller is not authorized to modify this account.")]
    Unauthorized,
    #[msg("Planet coordinates are out of range.")]
    InvalidCoordinates,
    #[msg("Planet count overflowed.")]
    PlanetCountOverflow,
    #[msg("Build queue is busy.")]
    QueueBusy,
    #[msg("No free building fields are available.")]
    NoFields,
    #[msg("Insufficient metal.")]
    InsufficientMetal,
    #[msg("Insufficient crystal.")]
    InsufficientCrystal,
    #[msg("Insufficient deuterium.")]
    InsufficientDeuterium,
    #[msg("No build is currently queued.")]
    NoBuild,
    #[msg("The queued build has not finished yet.")]
    BuildNotFinished,
    #[msg("Invalid research technology.")]
    InvalidTech,
    #[msg("Research lab level is too low.")]
    LabTooLow,
    #[msg("Research queue is busy.")]
    ResearchQueueBusy,
    #[msg("No research is currently queued.")]
    NoResearch,
    #[msg("The queued research has not finished yet.")]
    ResearchNotFinished,
    #[msg("Mission is invalid for this instruction.")]
    InvalidMission,
    #[msg("Mission arguments are invalid.")]
    InvalidArgs,
    #[msg("Invalid ship type.")]
    InvalidShipType,
    #[msg("Ship is locked by research requirements.")]
    TechLocked,
    #[msg("Shipyard level is too low.")]
    ShipyardTooLow,
    #[msg("Shipyard queue is busy.")]
    ShipyardQueueBusy,
    #[msg("No ship build is currently queued.")]
    NoShipBuild,
    #[msg("The queued ship build has not finished yet.")]
    ShipBuildNotFinished,
    #[msg("The selected fleet is empty.")]
    EmptyFleet,
    #[msg("No free mission slot is available.")]
    NoMissionSlot,
    #[msg("Insufficient ships are available.")]
    InsufficientShips,
    #[msg("Cargo exceeds the selected fleet capacity.")]
    ExceedsCargo,
    #[msg("Insufficient resources are available.")]
    InsufficientResources,
    #[msg("Mission slot is invalid.")]
    InvalidMissionSlot,
    #[msg("Mission destination does not match the provided destination planet.")]
    InvalidDestination,
    #[msg("Mission is still in flight.")]
    MissionInFlight,
    #[msg("Return trip has not completed yet.")]
    ReturnInFlight,
    #[msg("Mission was already resolved.")]
    AlreadyResolved,
    #[msg("Colonize mission is missing a colony ship.")]
    MissingColonyShip,
    #[msg("The provided vault authorization is invalid.")]
    InvalidVaultAuthorization,
    #[msg("The provided vault authorization has expired.")]
    VaultAuthorizationExpired,
    #[msg("The provided vault authorization was revoked.")]
    VaultAuthorizationRevoked,
    #[msg("Encrypted vault backup is too large.")]
    BackupTooLarge,
    #[msg("Transfer target has not initialized a player profile.")]
    TransferTargetNotInitialized,
    #[msg("The provided ANTIMATTER mint is invalid.")]
    InvalidAntimatterMint,
    #[msg("The provided ANTIMATTER mint must use 6 decimals.")]
    InvalidAntimatterMintDecimals,
    #[msg("The provided ANTIMATTER token account is invalid.")]
    InvalidAntimatterAccount,
    #[msg("Insufficient ANTIMATTER tokens.")]
    InsufficientAntimatter,
    #[msg("There is no remaining time to accelerate.")]
    NoAccelerationNeeded,
    #[msg("The ANTIMATTER burn amount overflowed.")]
    AntimatterAmountOverflow,
    #[msg("Only the authorized market PDA may settle market resources.")]
    UnauthorizedMarket,
}
