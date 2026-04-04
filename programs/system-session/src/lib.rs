use bolt_lang::*;
use component_planet::Planet;
use component_resources::Resources;
use component_fleet::Fleet;

declare_id!("BHRu4DADM4NsJvnvqY5znDUsrdvTrnkKyee9eYZ7Yd9G");

#[system]
pub mod system_session {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require!(!args.is_empty(), SessionError::InvalidArgs);
        require!(args[0] == 0, SessionError::InvalidArgs);
        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub planet:    Planet,
        pub resources: Resources,
        pub fleet:     Fleet,
    }
}

#[error_code]
pub enum SessionError {
    #[msg("Invalid args — args[0] must be 0 (end_session)")]
    InvalidArgs,
    #[msg("Authority must sign the session instruction")]
    Unauthorized,
}