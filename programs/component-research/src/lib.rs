use bolt_lang::*;

declare_id!("71Mk6jkgm6SuYBm4GasMHspcNL5vWJ3jhckjtZuYQ9Bu");

#[component(delegate)]
pub struct Research {
    pub creator:            Pubkey,
    pub energy_tech:        u8,
    pub combustion_drive:   u8,
    pub impulse_drive:      u8,
    pub hyperspace_drive:   u8,
    pub computer_tech:      u8,
    pub astrophysics:       u8,
    pub igr_network:        u8,
    pub queue_item:         u8,
    pub queue_target:       u8,
    pub research_finish_ts: i64,
}

impl Default for Research {
    fn default() -> Self {
        Self {
            bolt_metadata:       Default::default(),
            creator:             Pubkey::default(),
            energy_tech:         0,
            combustion_drive:    0,
            impulse_drive:       0,
            hyperspace_drive:    0,
            computer_tech:       0,
            astrophysics:        0,
            igr_network:         0,
            queue_item:          255,
            queue_target:        0,
            research_finish_ts:  0,
        }
    }
}

impl Research {
    pub fn level(&self, idx: u8) -> u8 {
        match idx {
            0 => self.energy_tech,
            1 => self.combustion_drive,
            2 => self.impulse_drive,
            3 => self.hyperspace_drive,
            4 => self.computer_tech,
            5 => self.astrophysics,
            6 => self.igr_network,
            _ => 0,
        }
    }

    pub fn set_level(&mut self, idx: u8, level: u8) {
        match idx {
            0 => self.energy_tech = level,
            1 => self.combustion_drive = level,
            2 => self.impulse_drive = level,
            3 => self.hyperspace_drive = level,
            4 => self.computer_tech = level,
            5 => self.astrophysics = level,
            6 => self.igr_network = level,
            _ => {}
        }
    }
}
