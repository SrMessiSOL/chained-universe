use bolt_lang::*;

declare_id!("8qbBLEdrN6qC1fFJQLM7a6Jqf2xfoDNfSmTQopMELSGm");

fn atk(lf:u32,hf:u32,cr:u32,bs:u32,bc:u32,bm:u32,ds:u32,de:u32)->u64{
    lf as u64*50+hf as u64*150+cr as u64*400+bs as u64*1_000
    +bc as u64*700+bm as u64*1_000+ds as u64*2_000+de as u64*200_000
}
fn shd(lf:u32,hf:u32,cr:u32,bs:u32,bc:u32,bm:u32,ds:u32,de:u32)->u64{
    lf as u64*10+hf as u64*25+cr as u64*50+bs as u64*200
    +bc as u64*400+bm as u64*500+ds as u64*500+de as u64*50_000
}
fn hul(lf:u32,hf:u32,cr:u32,bs:u32,bc:u32,bm:u32,ds:u32,de:u32)->u64{
    lf as u64*800+hf as u64*3_000+cr as u64*13_500+bs as u64*30_000
    +bc as u64*35_000+bm as u64*30_000+ds as u64*55_000+de as u64*2_000_000
}
fn sc(n:u32,r:u64)->u32{((n as u64*r)/1_000)as u32}

// FIX 2: Inline settle logic — Resources has no .settle() method.
// Mirrors the settle_resources() free function in system-build.
fn settle_resources(res: &mut component_resources::Resources, now: i64) {
    if res.last_update_ts <= 0 || now <= res.last_update_ts {
        res.last_update_ts = now;
        return;
    }
    let dt = (now - res.last_update_ts) as u64;
    let eff_num = if res.energy_consumption == 0 {
        10_000u64
    } else {
        (res.energy_production * 10_000 / res.energy_consumption).min(10_000)
    };
    let add_res = |current: u64, rate_per_hour: u64, cap: u64| -> u64 {
        let produced = rate_per_hour
            .saturating_mul(dt)
            .saturating_mul(eff_num)
            / 3600
            / 10_000;
        current.saturating_add(produced).min(cap)
    };
    res.metal     = add_res(res.metal,     res.metal_hour,     res.metal_cap);
    res.crystal   = add_res(res.crystal,   res.crystal_hour,   res.crystal_cap);
    res.deuterium = add_res(res.deuterium, res.deuterium_hour, res.deuterium_cap);
    res.last_update_ts = now;
}

// FIX 3: Inline cargo capacity — Mission has no .total_cargo_capacity() method.
// Formula mirrors system-launch: sc*5_000 + lc*25_000 + recycler*20_000
// + cruiser*800 + battleship*1_500.
fn mission_cargo_capacity(m: &component_fleet::Mission) -> u64 {
    m.s_small_cargo as u64 * 5_000
        + m.s_large_cargo as u64 * 25_000
        + m.s_recycler as u64 * 20_000
        + m.s_cruiser as u64 * 800
        + m.s_battleship as u64 * 1_500
}

#[system]
pub mod system_attack {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {

        require!(args.len() >= 9, E::InvalidArgs);
        let slot = args[0] as usize;
        let now  = i64::from_le_bytes(args[1..9].try_into().unwrap());

        // FIX 1: MAX_MISSIONS doesn't exist — Fleet.missions is [Mission; 4].
        require!(slot < 4, E::InvalidSlot);

        let (mt, applied, arrive_ts) = {
            let m = &ctx.accounts.attacker_fleet.missions[slot];
            (m.mission_type, m.applied, m.arrive_ts)
        };
        require!(mt == 1,          E::NotAttack);
        require!(!applied,         E::AlreadyApplied);
        require!(now >= arrive_ts, E::NotArrived);

        // FIX 2: Use the inlined free function instead of the non-existent method.
        settle_resources(&mut ctx.accounts.defender_resources, now);

        // Attacker stats
        let (alf,ahf,acr,abs,abc,abm,ads,ade,asc,alc) = {
            let m = &ctx.accounts.attacker_fleet.missions[slot];
            (m.s_light_fighter,m.s_heavy_fighter,m.s_cruiser,m.s_battleship,
             m.s_battlecruiser,m.s_bomber,m.s_destroyer,m.s_deathstar,
             m.s_small_cargo,m.s_large_cargo)
        };
        // Defender stats
        let (dlf,dhf,dcr,dbs,dbc,dbm,dds,dde) = {
            let f = &ctx.accounts.defender_fleet;
            (f.light_fighter,f.heavy_fighter,f.cruiser,f.battleship,
             f.battlecruiser,f.bomber,f.destroyer,f.deathstar)
        };

        let mut ah = hul(alf,ahf,acr,abs,abc,abm,ads,ade)+asc as u64*800+alc as u64*3_500;
        let mut as_ = shd(alf,ahf,acr,abs,abc,abm,ads,ade);
        let mut dh = hul(dlf,dhf,dcr,dbs,dbc,dbm,dds,dde);
        let mut ds_ = shd(dlf,dhf,dcr,dbs,dbc,dbm,dds,dde);
        let (tah, tas, tdh, tds) = (ah, as_, dh, ds_);

        let mut rlf=alf;let mut rhf=ahf;let mut rcr=acr;let mut rbs=abs;
        let mut rbc=abc;let mut rbm=abm;let mut rds=ads;let mut rde=ade;
        let mut rsc=asc;let mut rlc=alc;
        let mut dlf2=dlf;let mut dhf2=dhf;let mut dcr2=dcr;let mut dbs2=dbs;
        let mut dbc2=dbc;let mut dbm2=dbm;let mut dds2=dds;let mut dde2=dde;

        let mut rounds = 0u8;
        while rounds < 6 {
            let dead_a = ah==0;
            let dead_d = dh==0||(dlf2==0&&dhf2==0&&dcr2==0&&dbs2==0&&dbc2==0&&dbm2==0&&dds2==0&&dde2==0);
            if dead_a || dead_d { break; }

            // Attacker fires at defender
            let ad = atk(rlf,rhf,rcr,rbs,rbc,rbm,rds,rde)+rsc as u64*5+rlc as u64*5;
            if ds_ >= ad { ds_-=ad; } else { dh=dh.saturating_sub(ad-ds_); ds_=0; }
            if tdh>0 { let r=dh.saturating_mul(1_000)/tdh; dlf2=sc(dlf2,r);dhf2=sc(dhf2,r);dcr2=sc(dcr2,r);dbs2=sc(dbs2,r);dbc2=sc(dbc2,r);dbm2=sc(dbm2,r);dds2=sc(dds2,r);dde2=sc(dde2,r); }

            // Defender fires at attacker
            let dd = atk(dlf2,dhf2,dcr2,dbs2,dbc2,dbm2,dds2,dde2);
            if as_ >= dd { as_-=dd; } else { ah=ah.saturating_sub(dd-as_); as_=0; }
            if tah>0 { let r=ah.saturating_mul(1_000)/tah; rlf=sc(rlf,r);rhf=sc(rhf,r);rcr=sc(rcr,r);rbs=sc(rbs,r);rbc=sc(rbc,r);rbm=sc(rbm,r);rds=sc(rds,r);rde=sc(rde,r);rsc=sc(rsc,r);rlc=sc(rlc,r); }

            // Shield regen
            as_=tas; ds_=tds;
            rounds+=1;
        }

        let attacker_wins = ah>0 && (dh==0||(dlf2==0&&dhf2==0&&dcr2==0&&dbs2==0&&dbc2==0&&dbm2==0&&dds2==0&&dde2==0));

        ctx.accounts.defender_fleet.light_fighter=dlf2; ctx.accounts.defender_fleet.heavy_fighter=dhf2;
        ctx.accounts.defender_fleet.cruiser=dcr2; ctx.accounts.defender_fleet.battleship=dbs2;
        ctx.accounts.defender_fleet.battlecruiser=dbc2; ctx.accounts.defender_fleet.bomber=dbm2;
        ctx.accounts.defender_fleet.destroyer=dds2; ctx.accounts.defender_fleet.deathstar=dde2;

        // Update surviving attacker ships in mission slot
        {
            let m = &mut ctx.accounts.attacker_fleet.missions[slot];
            m.s_light_fighter=rlf; m.s_heavy_fighter=rhf; m.s_cruiser=rcr;
            m.s_battleship=rbs; m.s_battlecruiser=rbc; m.s_bomber=rbm;
            m.s_destroyer=rds; m.s_deathstar=rde;
            m.s_small_cargo=rsc; m.s_large_cargo=rlc;
        }

        if attacker_wins {
            // FIX 3: Use the inlined free function instead of the non-existent method.
            let cap = mission_cargo_capacity(&ctx.accounts.attacker_fleet.missions[slot]);
            let sm=(ctx.accounts.defender_resources.metal/2).min(cap/3);
            let sc2=(ctx.accounts.defender_resources.crystal/2).min(cap/3);
            let sd=(ctx.accounts.defender_resources.deuterium/2).min(cap/3);
            ctx.accounts.defender_resources.metal-=sm;
            ctx.accounts.defender_resources.crystal-=sc2;
            ctx.accounts.defender_resources.deuterium-=sd;
            let m=&mut ctx.accounts.attacker_fleet.missions[slot];
            m.cargo_metal=sm; m.cargo_crystal=sc2; m.cargo_deuterium=sd;
        }

        // Set return ETA — ships will fly back
        {
            let m=&mut ctx.accounts.attacker_fleet.missions[slot];
            m.applied=true;
            m.return_ts=now+(m.arrive_ts-m.depart_ts);
        }

        // Return ships to stationed fleet and clear mission slot
        {
            let return_ts = ctx.accounts.attacker_fleet.missions[slot].return_ts;
            if now >= return_ts {
                let m = &ctx.accounts.attacker_fleet.missions[slot];
                let (ret_lf,ret_hf,ret_cr,ret_bs,ret_bc,ret_bm,ret_ds,ret_de,ret_sc,ret_lc) =
                    (m.s_light_fighter,m.s_heavy_fighter,m.s_cruiser,m.s_battleship,
                     m.s_battlecruiser,m.s_bomber,m.s_destroyer,m.s_deathstar,
                     m.s_small_cargo,m.s_large_cargo);
                let (cm,cc,cd) = (m.cargo_metal,m.cargo_crystal,m.cargo_deuterium);

                let f = &mut ctx.accounts.attacker_fleet;
                f.light_fighter   = f.light_fighter.saturating_add(ret_lf);
                f.heavy_fighter   = f.heavy_fighter.saturating_add(ret_hf);
                f.cruiser         = f.cruiser.saturating_add(ret_cr);
                f.battleship      = f.battleship.saturating_add(ret_bs);
                f.battlecruiser   = f.battlecruiser.saturating_add(ret_bc);
                f.bomber          = f.bomber.saturating_add(ret_bm);
                f.destroyer       = f.destroyer.saturating_add(ret_ds);
                f.deathstar       = f.deathstar.saturating_add(ret_de);
                f.small_cargo     = f.small_cargo.saturating_add(ret_sc);
                f.large_cargo     = f.large_cargo.saturating_add(ret_lc);
                // Credit looted resources
                ctx.accounts.attacker_resources.metal     = ctx.accounts.attacker_resources.metal.saturating_add(cm);
                ctx.accounts.attacker_resources.crystal   = ctx.accounts.attacker_resources.crystal.saturating_add(cc);
                ctx.accounts.attacker_resources.deuterium = ctx.accounts.attacker_resources.deuterium.saturating_add(cd);
                // Clear the slot
                ctx.accounts.attacker_fleet.missions[slot] = component_fleet::Mission::default();
                ctx.accounts.attacker_fleet.active_missions =
                    ctx.accounts.attacker_fleet.active_missions.saturating_sub(1);
            }
        }

        emit!(BattleResult{attacker_wins,rounds});
        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub attacker_fleet:     component_fleet::Fleet,
        pub attacker_resources: component_resources::Resources,
        pub defender_fleet:     component_fleet::Fleet,
        pub defender_resources: component_resources::Resources,
    }
}

#[event]
pub struct BattleResult { pub attacker_wins: bool, pub rounds: u8 }

#[error_code]
pub enum E {
    #[msg("Invalid args")]      InvalidArgs,
    #[msg("Invalid slot")]      InvalidSlot,
    #[msg("Not an attack")]     NotAttack,
    #[msg("Already applied")]   AlreadyApplied,
    #[msg("Not arrived")]       NotArrived,
}