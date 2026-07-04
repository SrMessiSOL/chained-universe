use anchor_lang::prelude::*;

declare_id!("P8gtUdeBpt6oG5a7LFQbxnHJbLyc9X9BhTj8iz8pSn1");

pub const MAX_PRIVATE_PLANET_NAME_LEN: usize = 32;
pub const MAX_ENCRYPTED_PRIVATE_STATE_LEN: usize = 1536;
pub const MAX_ENCRYPTED_SPY_REPORT_LEN: usize = 1024;
pub const MAX_REVEAL_LEVEL: u8 = 4;
pub const PRIVATE_STATE_SCHEMA_V1: u8 = 1;
pub const PRIVACY_ENGINE_COMMITMENT_ONLY: u8 = 0;
pub const PRIVACY_ENGINE_CLIENT_AES_GCM: u8 = 1;
pub const PRIVATE_CIPHERTEXT_SCHEMA_V1: u16 = 1;
pub const PRIVATE_PLANET_STATE_SPACE: usize = 8 + PrivatePlanetState::INIT_SPACE;
pub const SPY_REPORT_REQUEST_SPACE: usize = 8 + SpyReportRequest::INIT_SPACE;
pub const SPY_REPORT_SPACE: usize = 8 + SpyReport::INIT_SPACE;
pub const GAME_STATE_PROGRAM_ID: Pubkey = pubkey!("FJGxh6SKgNoTVzHj98oBsC2oaEy8ovadVJf8rDUNaEHb");
pub const GAME_PUBLIC_PLANET_STATE_DISCRIMINATOR: [u8; 8] = [61, 168, 213, 170, 12, 18, 66, 158];
pub const GAME_PUBLIC_PLANET_AUTHORITY_OFFSET: usize = 8;
pub const GAME_PUBLIC_PLANET_AUTHORITY_END: usize = GAME_PUBLIC_PLANET_AUTHORITY_OFFSET + 32;
pub const GAME_PUBLIC_PLANET_GALAXY_OFFSET: usize = GAME_PUBLIC_PLANET_AUTHORITY_END + 32 + 4;
pub const GAME_PUBLIC_PLANET_SYSTEM_OFFSET: usize = GAME_PUBLIC_PLANET_GALAXY_OFFSET + 2;
pub const GAME_PUBLIC_PLANET_POSITION_OFFSET: usize = GAME_PUBLIC_PLANET_SYSTEM_OFFSET + 2;
pub const GAME_PUBLIC_PLANET_VERSION_OFFSET: usize = GAME_PUBLIC_PLANET_POSITION_OFFSET + 1;
pub const GAME_PUBLIC_PLANET_MIN_LEN: usize = GAME_PUBLIC_PLANET_VERSION_OFFSET + 1;
pub const GAME_PUBLIC_PLANET_V2: u8 = 2;

#[program]
pub mod private_state {
    use super::*;

    pub fn initialize_private_planet(
        ctx: Context<InitializePrivatePlanet>,
        galaxy: u16,
        system: u16,
        position: u8,
        name: [u8; MAX_PRIVATE_PLANET_NAME_LEN],
        digest: PrivateStateDigest,
        encrypted_state: EncryptedPrivateStateBlob,
    ) -> Result<()> {
        require!(
            galaxy > 0 && system > 0 && position > 0,
            PrivateStateError::InvalidCoordinates
        );
        validate_private_state_digest(&digest)?;
        validate_encrypted_private_state(&digest, &encrypted_state)?;
        assert_public_game_planet(
            &ctx.accounts.public_planet,
            &ctx.accounts.authority.key(),
            galaxy,
            system,
            position,
        )?;
        let now = Clock::get()?.unix_timestamp;
        ctx.accounts.private_planet.set_inner(PrivatePlanetState {
            authority: ctx.accounts.authority.key(),
            public_planet: ctx.accounts.public_planet.key(),
            galaxy,
            system,
            position,
            name,
            created_at: now,
            public_protection_until_ts: now.saturating_add(7 * 24 * 60 * 60),
            schema_version: PRIVATE_STATE_SCHEMA_V1,
            state_epoch: 0,
            state_hash: digest.state_hash,
            encrypted_state_hash: digest.encrypted_state_hash,
            privacy_engine: digest.seal.privacy_engine,
            ciphertext_schema: digest.seal.ciphertext_schema,
            encryption_key_hash: digest.seal.encryption_key_hash,
            decrypt_policy_hash: digest.seal.decrypt_policy_hash,
            encrypted_state_ciphertext: encrypted_state.ciphertext,
            ciphertext_nonce: encrypted_state.nonce,
            ciphertext_recovery_salt: encrypted_state.recovery_salt,
            ciphertext_kdf_salt: encrypted_state.kdf_salt,
            resources_commitment: digest.commitments.resources,
            buildings_commitment: digest.commitments.buildings,
            research_commitment: digest.commitments.research,
            fleet_commitment: digest.commitments.fleet,
            defense_commitment: digest.commitments.defense,
            last_transition_hash: [0; 32],
            last_action_kind: 0,
            report_nonce: 0,
            bump: ctx.bumps.private_planet,
        });
        Ok(())
    }

    pub fn rotate_private_commitments(
        ctx: Context<RotatePrivateCommitments>,
        new_digest: PrivateStateDigest,
        encrypted_state: EncryptedPrivateStateBlob,
        transition_hash: [u8; 32],
        action_kind: u8,
    ) -> Result<()> {
        require!(
            transition_hash != [0; 32],
            PrivateStateError::InvalidTransitionHash
        );
        require!(action_kind > 0, PrivateStateError::InvalidActionKind);
        validate_private_state_digest(&new_digest)?;
        validate_encrypted_private_state(&new_digest, &encrypted_state)?;
        let planet = &mut ctx.accounts.private_planet;
        planet.state_epoch = planet
            .state_epoch
            .checked_add(1)
            .ok_or(PrivateStateError::EpochOverflow)?;
        planet.schema_version = PRIVATE_STATE_SCHEMA_V1;
        planet.state_hash = new_digest.state_hash;
        planet.encrypted_state_hash = new_digest.encrypted_state_hash;
        planet.privacy_engine = new_digest.seal.privacy_engine;
        planet.ciphertext_schema = new_digest.seal.ciphertext_schema;
        planet.encryption_key_hash = new_digest.seal.encryption_key_hash;
        planet.decrypt_policy_hash = new_digest.seal.decrypt_policy_hash;
        planet.encrypted_state_ciphertext = encrypted_state.ciphertext;
        planet.ciphertext_nonce = encrypted_state.nonce;
        planet.ciphertext_recovery_salt = encrypted_state.recovery_salt;
        planet.ciphertext_kdf_salt = encrypted_state.kdf_salt;
        planet.resources_commitment = new_digest.commitments.resources;
        planet.buildings_commitment = new_digest.commitments.buildings;
        planet.research_commitment = new_digest.commitments.research;
        planet.fleet_commitment = new_digest.commitments.fleet;
        planet.defense_commitment = new_digest.commitments.defense;
        planet.last_transition_hash = transition_hash;
        planet.last_action_kind = action_kind;
        emit!(PrivateStateRotatedEvent {
            private_planet: planet.key(),
            authority: planet.authority,
            state_epoch: planet.state_epoch,
            transition_hash,
            action_kind,
        });
        Ok(())
    }

    pub fn publish_spy_report(
        ctx: Context<PublishSpyReport>,
        encrypted_report: EncryptedSpyReportBlob,
        report_commitment: [u8; 32],
    ) -> Result<()> {
        let request = &ctx.accounts.spy_report_request;
        validate_encrypted_spy_report(&encrypted_report)?;
        require!(
            report_commitment != [0; 32],
            PrivateStateError::InvalidReportHash
        );

        let planet = &mut ctx.accounts.private_planet;
        require!(
            request.target_epoch == planet.state_epoch,
            PrivateStateError::StaleSpyRequest
        );

        let now = Clock::get()?.unix_timestamp;
        ctx.accounts.spy_report.set_inner(SpyReport {
            target_planet: planet.key(),
            target_authority: planet.authority,
            spy_authority: ctx.accounts.spy_authority.key(),
            resolver: ctx.accounts.resolver.key(),
            target_epoch: planet.state_epoch,
            report_nonce: request.report_nonce,
            reveal_level: request.reveal_level_cap,
            report_ciphertext_hash: report_commitment,
            report_ciphertext: encrypted_report.ciphertext,
            report_nonce_bytes: encrypted_report.nonce,
            report_commitment,
            created_at: now,
            bump: ctx.bumps.spy_report,
        });

        emit!(SpyReportPublishedEvent {
            target_planet: planet.key(),
            spy_authority: ctx.accounts.spy_authority.key(),
            resolver: ctx.accounts.resolver.key(),
            target_epoch: planet.state_epoch,
            report_nonce: request.report_nonce,
            reveal_level: request.reveal_level_cap,
            report_ciphertext_hash: ctx.accounts.spy_report.report_ciphertext_hash,
            report_commitment,
        });
        Ok(())
    }

    pub fn request_spy_report(
        ctx: Context<RequestSpyReport>,
        reveal_level_cap: u8,
        encrypted_input_hash: [u8; 32],
        request_commitment: [u8; 32],
    ) -> Result<()> {
        require!(
            reveal_level_cap <= MAX_REVEAL_LEVEL,
            PrivateStateError::InvalidRevealLevel
        );
        require!(
            encrypted_input_hash != [0; 32] && request_commitment != [0; 32],
            PrivateStateError::InvalidSpyRequest
        );
        require!(
            ctx.accounts.resolver.key() != Pubkey::default(),
            PrivateStateError::InvalidResolver
        );

        let target = &mut ctx.accounts.private_planet;
        let nonce = target.report_nonce;
        target.report_nonce = target
            .report_nonce
            .checked_add(1)
            .ok_or(PrivateStateError::NonceOverflow)?;
        let now = Clock::get()?.unix_timestamp;

        ctx.accounts.spy_report_request.set_inner(SpyReportRequest {
            target_planet: target.key(),
            target_authority: target.authority,
            spy_authority: ctx.accounts.spy_authority.key(),
            resolver: ctx.accounts.resolver.key(),
            target_epoch: target.state_epoch,
            report_nonce: nonce,
            reveal_level_cap,
            encrypted_input_hash,
            request_commitment,
            created_at: now,
            resolved: false,
            bump: ctx.bumps.spy_report_request,
        });

        emit!(SpyReportRequestedEvent {
            target_planet: target.key(),
            spy_authority: ctx.accounts.spy_authority.key(),
            resolver: ctx.accounts.resolver.key(),
            target_epoch: target.state_epoch,
            report_nonce: nonce,
            reveal_level_cap,
            encrypted_input_hash,
            request_commitment,
        });
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivateCommitments {
    pub resources: [u8; 32],
    pub buildings: [u8; 32],
    pub research: [u8; 32],
    pub fleet: [u8; 32],
    pub defense: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivateStateDigest {
    pub state_hash: [u8; 32],
    pub encrypted_state_hash: [u8; 32],
    pub seal: PrivateStateSeal,
    pub commitments: PrivateCommitments,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivateStateSeal {
    pub privacy_engine: u8,
    pub ciphertext_schema: u16,
    pub encryption_key_hash: [u8; 32],
    pub decrypt_policy_hash: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct EncryptedPrivateStateBlob {
    pub nonce: [u8; 12],
    pub recovery_salt: [u8; 16],
    pub kdf_salt: [u8; 16],
    pub ciphertext: Vec<u8>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct EncryptedSpyReportBlob {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

#[account]
#[derive(InitSpace)]
pub struct PrivatePlanetState {
    pub authority: Pubkey,
    pub public_planet: Pubkey,
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub name: [u8; MAX_PRIVATE_PLANET_NAME_LEN],
    pub created_at: i64,
    pub public_protection_until_ts: i64,
    pub schema_version: u8,
    pub state_epoch: u64,
    pub state_hash: [u8; 32],
    pub encrypted_state_hash: [u8; 32],
    pub privacy_engine: u8,
    pub ciphertext_schema: u16,
    pub encryption_key_hash: [u8; 32],
    pub decrypt_policy_hash: [u8; 32],
    #[max_len(MAX_ENCRYPTED_PRIVATE_STATE_LEN)]
    pub encrypted_state_ciphertext: Vec<u8>,
    pub ciphertext_nonce: [u8; 12],
    pub ciphertext_recovery_salt: [u8; 16],
    pub ciphertext_kdf_salt: [u8; 16],
    pub resources_commitment: [u8; 32],
    pub buildings_commitment: [u8; 32],
    pub research_commitment: [u8; 32],
    pub fleet_commitment: [u8; 32],
    pub defense_commitment: [u8; 32],
    pub last_transition_hash: [u8; 32],
    pub last_action_kind: u8,
    pub report_nonce: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct SpyReport {
    pub target_planet: Pubkey,
    pub target_authority: Pubkey,
    pub spy_authority: Pubkey,
    pub resolver: Pubkey,
    pub target_epoch: u64,
    pub report_nonce: u64,
    pub reveal_level: u8,
    pub report_ciphertext_hash: [u8; 32],
    #[max_len(MAX_ENCRYPTED_SPY_REPORT_LEN)]
    pub report_ciphertext: Vec<u8>,
    pub report_nonce_bytes: [u8; 12],
    pub report_commitment: [u8; 32],
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct SpyReportRequest {
    pub target_planet: Pubkey,
    pub target_authority: Pubkey,
    pub spy_authority: Pubkey,
    pub resolver: Pubkey,
    pub target_epoch: u64,
    pub report_nonce: u64,
    pub reveal_level_cap: u8,
    pub encrypted_input_hash: [u8; 32],
    pub request_commitment: [u8; 32],
    pub created_at: i64,
    pub resolved: bool,
    pub bump: u8,
}

#[derive(Accounts)]
#[instruction(galaxy: u16, system: u16, position: u8)]
pub struct InitializePrivatePlanet<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: public game-state V2 planet account. The handler verifies owner, discriminator, authority, and coordinates.
    pub public_planet: UncheckedAccount<'info>,
    #[account(
        init,
        payer = authority,
        space = PRIVATE_PLANET_STATE_SPACE,
        seeds = [
            b"private-planet",
            public_planet.key().as_ref(),
        ],
        bump
    )]
    pub private_planet: Account<'info, PrivatePlanetState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RotatePrivateCommitments<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority @ PrivateStateError::Unauthorized)]
    pub private_planet: Account<'info, PrivatePlanetState>,
}

#[derive(Accounts)]
pub struct PublishSpyReport<'info> {
    #[account(mut)]
    pub resolver: Signer<'info>,
    #[account(mut)]
    /// CHECK: original spy authority from the request; receives closed request rent.
    pub spy_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub private_planet: Account<'info, PrivatePlanetState>,
    #[account(
        mut,
        has_one = spy_authority @ PrivateStateError::Unauthorized,
        has_one = resolver @ PrivateStateError::InvalidResolver,
        constraint = spy_report_request.target_planet == private_planet.key() @ PrivateStateError::InvalidSpyRequest,
        close = spy_authority
    )]
    pub spy_report_request: Account<'info, SpyReportRequest>,
    #[account(
        init,
        payer = resolver,
        space = SPY_REPORT_SPACE,
        seeds = [
            b"spy-report",
            private_planet.key().as_ref(),
            spy_authority.key().as_ref(),
            &spy_report_request.report_nonce.to_le_bytes(),
        ],
        bump
    )]
    pub spy_report: Account<'info, SpyReport>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestSpyReport<'info> {
    #[account(mut)]
    pub spy_authority: Signer<'info>,
    /// CHECK: expected internal privacy resolver or dev resolver.
    pub resolver: UncheckedAccount<'info>,
    #[account(mut)]
    pub private_planet: Account<'info, PrivatePlanetState>,
    #[account(
        init,
        payer = spy_authority,
        space = SPY_REPORT_REQUEST_SPACE,
        seeds = [
            b"spy-report-request",
            private_planet.key().as_ref(),
            spy_authority.key().as_ref(),
            &private_planet.report_nonce.to_le_bytes(),
        ],
        bump
    )]
    pub spy_report_request: Account<'info, SpyReportRequest>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct PrivateStateRotatedEvent {
    pub private_planet: Pubkey,
    pub authority: Pubkey,
    pub state_epoch: u64,
    pub transition_hash: [u8; 32],
    pub action_kind: u8,
}

#[event]
pub struct SpyReportPublishedEvent {
    pub target_planet: Pubkey,
    pub spy_authority: Pubkey,
    pub resolver: Pubkey,
    pub target_epoch: u64,
    pub report_nonce: u64,
    pub reveal_level: u8,
    pub report_ciphertext_hash: [u8; 32],
    pub report_commitment: [u8; 32],
}

#[event]
pub struct SpyReportRequestedEvent {
    pub target_planet: Pubkey,
    pub spy_authority: Pubkey,
    pub resolver: Pubkey,
    pub target_epoch: u64,
    pub report_nonce: u64,
    pub reveal_level_cap: u8,
    pub encrypted_input_hash: [u8; 32],
    pub request_commitment: [u8; 32],
}

#[error_code]
pub enum PrivateStateError {
    Unauthorized,
    InvalidCoordinates,
    InvalidPublicPlanet,
    PublicPlanetAuthorityMismatch,
    PublicPlanetCoordinatesMismatch,
    InvalidRevealLevel,
    InvalidTransitionHash,
    InvalidReportHash,
    InvalidResolver,
    InvalidStateDigest,
    InvalidActionKind,
    InvalidPrivacyEngine,
    InvalidCiphertextSchema,
    InvalidDecryptPolicy,
    InvalidEncryptionKey,
    InvalidCiphertext,
    InvalidSpyRequest,
    StaleSpyRequest,
    EpochOverflow,
    NonceOverflow,
}

fn validate_private_state_digest(digest: &PrivateStateDigest) -> Result<()> {
    require!(
        digest.state_hash != [0; 32],
        PrivateStateError::InvalidStateDigest
    );
    require!(
        digest.encrypted_state_hash != [0; 32],
        PrivateStateError::InvalidStateDigest
    );
    validate_private_state_seal(&digest.seal)?;
    require!(
        digest.commitments.resources != [0; 32]
            && digest.commitments.buildings != [0; 32]
            && digest.commitments.research != [0; 32]
            && digest.commitments.fleet != [0; 32]
            && digest.commitments.defense != [0; 32],
        PrivateStateError::InvalidStateDigest
    );
    Ok(())
}

fn validate_private_state_seal(seal: &PrivateStateSeal) -> Result<()> {
    require!(
        seal.privacy_engine == PRIVACY_ENGINE_COMMITMENT_ONLY
            || seal.privacy_engine == PRIVACY_ENGINE_CLIENT_AES_GCM,
        PrivateStateError::InvalidPrivacyEngine
    );
    require!(
        seal.ciphertext_schema == PRIVATE_CIPHERTEXT_SCHEMA_V1,
        PrivateStateError::InvalidCiphertextSchema
    );
    require!(
        seal.decrypt_policy_hash != [0; 32],
        PrivateStateError::InvalidDecryptPolicy
    );
    if seal.privacy_engine == PRIVACY_ENGINE_CLIENT_AES_GCM {
        require!(
            seal.encryption_key_hash != [0; 32],
            PrivateStateError::InvalidEncryptionKey
        );
    }
    Ok(())
}

fn validate_encrypted_private_state(
    _digest: &PrivateStateDigest,
    encrypted_state: &EncryptedPrivateStateBlob,
) -> Result<()> {
    require!(
        !encrypted_state.ciphertext.is_empty()
            && encrypted_state.ciphertext.len() <= MAX_ENCRYPTED_PRIVATE_STATE_LEN,
        PrivateStateError::InvalidCiphertext
    );
    require!(
        encrypted_state.nonce != [0; 12]
            && encrypted_state.recovery_salt != [0; 16]
            && encrypted_state.kdf_salt != [0; 16],
        PrivateStateError::InvalidCiphertext
    );
    Ok(())
}

fn validate_encrypted_spy_report(encrypted_report: &EncryptedSpyReportBlob) -> Result<()> {
    require!(
        !encrypted_report.ciphertext.is_empty()
            && encrypted_report.ciphertext.len() <= MAX_ENCRYPTED_SPY_REPORT_LEN,
        PrivateStateError::InvalidReportHash
    );
    require!(
        encrypted_report.nonce != [0; 12],
        PrivateStateError::InvalidReportHash
    );
    Ok(())
}

fn assert_public_game_planet(
    public_planet: &UncheckedAccount,
    authority: &Pubkey,
    galaxy: u16,
    system: u16,
    position: u8,
) -> Result<()> {
    require_keys_eq!(
        *public_planet.owner,
        GAME_STATE_PROGRAM_ID,
        PrivateStateError::InvalidPublicPlanet
    );

    let data = public_planet.try_borrow_data()?;
    require!(
        data.len() >= GAME_PUBLIC_PLANET_MIN_LEN,
        PrivateStateError::InvalidPublicPlanet
    );
    require!(
        data[..8] == GAME_PUBLIC_PLANET_STATE_DISCRIMINATOR,
        PrivateStateError::InvalidPublicPlanet
    );

    let stored_authority = Pubkey::new_from_array(
        data[GAME_PUBLIC_PLANET_AUTHORITY_OFFSET..GAME_PUBLIC_PLANET_AUTHORITY_END]
            .try_into()
            .map_err(|_| error!(PrivateStateError::InvalidPublicPlanet))?,
    );
    require_keys_eq!(
        stored_authority,
        *authority,
        PrivateStateError::PublicPlanetAuthorityMismatch
    );

    let stored_galaxy = u16::from_le_bytes(
        data[GAME_PUBLIC_PLANET_GALAXY_OFFSET..GAME_PUBLIC_PLANET_GALAXY_OFFSET + 2]
            .try_into()
            .map_err(|_| error!(PrivateStateError::InvalidPublicPlanet))?,
    );
    let stored_system = u16::from_le_bytes(
        data[GAME_PUBLIC_PLANET_SYSTEM_OFFSET..GAME_PUBLIC_PLANET_SYSTEM_OFFSET + 2]
            .try_into()
            .map_err(|_| error!(PrivateStateError::InvalidPublicPlanet))?,
    );
    let stored_position = data[GAME_PUBLIC_PLANET_POSITION_OFFSET];
    let stored_version = data[GAME_PUBLIC_PLANET_VERSION_OFFSET];

    require!(
        stored_galaxy == galaxy && stored_system == system && stored_position == position,
        PrivateStateError::PublicPlanetCoordinatesMismatch
    );
    require!(
        stored_version == GAME_PUBLIC_PLANET_V2,
        PrivateStateError::InvalidPublicPlanet
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_seal() -> PrivateStateSeal {
        PrivateStateSeal {
            privacy_engine: PRIVACY_ENGINE_COMMITMENT_ONLY,
            ciphertext_schema: PRIVATE_CIPHERTEXT_SCHEMA_V1,
            encryption_key_hash: [8; 32],
            decrypt_policy_hash: [9; 32],
        }
    }

    #[test]
    fn private_commitments_roundtrip() {
        let commitments = PrivateCommitments {
            resources: [1; 32],
            buildings: [2; 32],
            research: [3; 32],
            fleet: [4; 32],
            defense: [5; 32],
        };
        let mut data = Vec::new();
        commitments.serialize(&mut data).unwrap();
        let decoded = PrivateCommitments::deserialize(&mut data.as_slice()).unwrap();
        assert_eq!(decoded, commitments);
    }

    #[test]
    fn account_space_includes_anchor_discriminator() {
        assert!(PRIVATE_PLANET_STATE_SPACE > 8);
        assert!(SPY_REPORT_SPACE > 8);
    }

    #[test]
    fn private_state_digest_rejects_zero_hashes() {
        let zero_digest = PrivateStateDigest {
            state_hash: [0; 32],
            encrypted_state_hash: [1; 32],
            seal: test_seal(),
            commitments: PrivateCommitments {
                resources: [1; 32],
                buildings: [1; 32],
                research: [1; 32],
                fleet: [1; 32],
                defense: [1; 32],
            },
        };
        assert!(validate_private_state_digest(&zero_digest).is_err());

        let valid_digest = PrivateStateDigest {
            state_hash: [2; 32],
            encrypted_state_hash: [3; 32],
            seal: test_seal(),
            commitments: PrivateCommitments {
                resources: [4; 32],
                buildings: [5; 32],
                research: [6; 32],
                fleet: [7; 32],
                defense: [8; 32],
            },
        };
        assert!(validate_private_state_digest(&valid_digest).is_ok());
    }

    #[test]
    fn client_encryption_seal_requires_key_hash_and_policy() {
        let mut seal = test_seal();
        seal.privacy_engine = PRIVACY_ENGINE_CLIENT_AES_GCM;
        seal.encryption_key_hash = [0; 32];
        assert!(validate_private_state_seal(&seal).is_err());

        seal.encryption_key_hash = [7; 32];
        assert!(validate_private_state_seal(&seal).is_ok());

        seal.decrypt_policy_hash = [0; 32];
        assert!(validate_private_state_seal(&seal).is_err());
    }

    #[test]
    fn encrypted_private_state_requires_ciphertext_and_salts() {
        let ciphertext = b"encrypted-state".to_vec();
        let digest = PrivateStateDigest {
            state_hash: [1; 32],
            encrypted_state_hash: [2; 32],
            seal: test_seal(),
            commitments: PrivateCommitments {
                resources: [1; 32],
                buildings: [2; 32],
                research: [3; 32],
                fleet: [4; 32],
                defense: [5; 32],
            },
        };
        let encrypted_state = EncryptedPrivateStateBlob {
            nonce: [6; 12],
            recovery_salt: [8; 16],
            kdf_salt: [7; 16],
            ciphertext,
        };

        assert!(validate_encrypted_private_state(&digest, &encrypted_state).is_ok());
        let empty_state = EncryptedPrivateStateBlob {
            nonce: [6; 12],
            recovery_salt: [8; 16],
            kdf_salt: [7; 16],
            ciphertext: Vec::new(),
        };
        assert!(validate_encrypted_private_state(&digest, &empty_state).is_err());
    }
}
