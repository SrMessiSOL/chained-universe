use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ResourceType {
    Metal = 0,
    Crystal = 1,
    Deuterium = 2,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Metal => "metal",
            ResourceType::Crystal => "crystal",
            ResourceType::Deuterium => "deuterium",
        }
    }
}
