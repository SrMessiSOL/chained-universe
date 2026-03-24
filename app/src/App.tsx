import React, { useState, useEffect, useCallback, useRef } from "react";
import { useConnection, useWallet, useAnchorWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { AnchorProvider } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

import { BUILDINGS, SHIPS, SHIP_TYPES } from "./constants";
import {
  SolarGridClient, GameAddresses,
  OnChainPlanet, OnChainResources, OnChainFleet, GalaxyEntry,
  formatNum, formatDuration, pendingProduction,
  buildCost, buildSeconds,
} from "./states/game";

type Page = "overview" | "buildings" | "shipyard" | "fleet" | "galaxy" | "messages";

export default function App() {
  const { connection } = useConnection();
  const wallet         = useAnchorWallet();
  const { connected, publicKey } = useWallet();

  const [page, setPage]           = useState<Page>("overview");
  const [addresses, setAddresses] = useState<GameAddresses | null>(null);
  const [planet, setPlanet]       = useState<OnChainPlanet | null>(null);
  const [resources, setResources] = useState<OnChainResources | null>(null);
  const [fleet, setFleet]         = useState<OnChainFleet | null>(null);
  const [status, setStatus]       = useState("");
  const [loading, setLoading]     = useState(false);
  const [blockNum, setBlockNum]   = useState(0);
  const [liveRes, setLiveRes]     = useState({ metal: 0, crystal: 0, deuterium: 0 });
  const [planetName, setPlanetName] = useState("Homeworld");
  const [showNameInput, setShowNameInput] = useState(false);

  const clientRef = useRef<SolarGridClient | null>(null);
  const pollRef   = useRef<ReturnType<typeof setInterval> | null>(null);

  // Build client when wallet connects
  useEffect(() => {
    if (!wallet || !connected || !publicKey) {
      clientRef.current = null;
      setAddresses(null); setPlanet(null); setResources(null); setFleet(null);
      return;
    }
    const provider = new AnchorProvider(connection, wallet, { commitment: "confirmed" });
    clientRef.current = new SolarGridClient(connection, provider);
    const saved = clientRef.current.loadAddresses(publicKey);
    setAddresses(saved || null);
    if (!saved) { setPlanet(null); setResources(null); setFleet(null); }
  }, [wallet, connected, publicKey, connection]);

  // Poll chain state every 6 s
  const poll = useCallback(async () => {
    if (!addresses || !clientRef.current) return;
    const client = clientRef.current;
    try {
      const [p, r, f] = await Promise.all([
        client.fetchPlanet(addresses.planetPda),
        client.fetchResources(addresses.resourcesPda),
        client.fetchFleet(addresses.fleetPda),
      ]);
      if (p) setPlanet(p);
      if (r) setResources(r);
      if (f) setFleet(f);
      const slot = await connection.getSlot();
      setBlockNum(slot);
    } catch (e) { console.warn("Poll:", e); }
  }, [addresses, connection]);

  useEffect(() => {
    if (!addresses) return;
    poll();
    pollRef.current = setInterval(poll, 6000);
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, [addresses, poll]);

  // Live resource ticker
  useEffect(() => {
    if (!resources) return;
    const id = setInterval(() => {
      const now  = Math.floor(Date.now() / 1000);
      const live = pendingProduction(resources, now);
      setLiveRes(live);
    }, 1000);
    return () => clearInterval(id);
  }, [resources]);

  // ── Handlers ──────────────────────────────────────────────────────────────

  const handleInit = useCallback(async () => {
    if (!clientRef.current) return;
    setLoading(true);
    setStatus("Creating your planet in the shared universe…");
    try {
      const addrs = await clientRef.current.initializeWorld(planetName);
      setAddresses(addrs);
      setStatus("✅ Planet created! Fetching state…");
      setShowNameInput(false);
      setTimeout(poll, 2500);
    } catch (e: any) {
      console.error(e);
      setStatus("❌ " + (e.message || String(e)));
    }
    setLoading(false);
  }, [planetName, poll]);

  const handleBuild = useCallback(async (idx: number, name: string) => {
    if (!clientRef.current || !addresses) return;
    setLoading(true);
    setStatus(`Upgrading ${name}…`);
    try {
      const sig = await clientRef.current.startBuild(addresses.worldPda, addresses.entityPda, idx);
      setStatus(`✅ ${name} upgrade started! ${sig.slice(0,8)}…`);
      setTimeout(poll, 2500);
    } catch (e: any) {
      console.error(e); setStatus("❌ " + (e.message || String(e)));
    }
    setLoading(false);
  }, [addresses, poll]);

  const handleFinishBuild = useCallback(async () => {
    if (!clientRef.current || !addresses) return;
    setLoading(true);
    setStatus("Completing upgrade…");
    try {
      await clientRef.current.finishBuild(addresses.worldPda, addresses.entityPda);
      setStatus("✅ Upgrade complete!");
      setTimeout(poll, 2500);
    } catch (e: any) {
      console.error(e); setStatus("❌ " + (e.message || String(e)));
    }
    setLoading(false);
  }, [addresses, poll]);

  const handleBuildShip = useCallback(async (shipKey: string, name: string, qty: number) => {
    if (!clientRef.current || !addresses) return;
    setLoading(true);
    setStatus(`Building ${qty}× ${name}…`);
    try {
      const shipType = SHIP_TYPES[shipKey];
      await clientRef.current.buildShip(addresses.worldPda, addresses.entityPda, shipType, qty);
      setStatus(`✅ ${qty}× ${name} built!`);
      setTimeout(poll, 2500);
    } catch (e: any) {
      console.error(e); setStatus("❌ " + (e.message || String(e)));
    }
    setLoading(false);
  }, [addresses, poll]);

  const handleSettleProduction = useCallback(async () => {
    if (!clientRef.current || !addresses) return;
    setLoading(true);
    setStatus("Settling production on-chain…");
    try {
      const sig = await clientRef.current.settleProduction(addresses.worldPda, addresses.entityPda);
      setStatus(`✅ Production settled! ${sig.slice(0,8)}…`);
      setTimeout(poll, 2500);
    } catch (e: any) {
      console.error(e); setStatus("❌ " + (e.message || String(e)));
    }
    setLoading(false);
  }, [addresses, poll]);

  const energy = resources ? resources.energyProduction - resources.energyConsumption : 0;
  const displayRes = resources ? liveRes : { metal: 0, crystal: 0, deuterium: 0 };

  return (
    <div style={{ display:"flex", flexDirection:"column", height:"100vh" }}>
      {/* ── Header ── */}
      <header style={{ display:"flex", alignItems:"center", justifyContent:"space-between", padding:"0 20px", height:52, background:"rgba(2,8,20,0.97)", borderBottom:"1px solid var(--border)", flexShrink:0 }}>
        <div style={{ fontFamily:"'Orbitron',sans-serif", fontWeight:900, fontSize:"1.2rem", color:"var(--accent)", letterSpacing:"0.1em" }}>
          SOLAR<span style={{color:"var(--accent2)"}}>GRID</span>
          <span style={{ fontSize:"0.5rem", color:"var(--dim)", marginLeft:10, letterSpacing:"0.1em" }}>BOLT ECS · DEVNET</span>
        </div>
        <div style={{ fontFamily:"'Share Tech Mono',monospace", fontSize:"0.68rem", color:"var(--dim)", display:"flex", alignItems:"center", gap:16 }}>
          {blockNum > 0 && <span>Block: <span style={{color:"var(--accent)"}}>{blockNum.toLocaleString()}</span></span>}
          {status && (
            <span style={{ color: status.startsWith("❌") ? "var(--red)" : status.startsWith("✅") ? "var(--accent3)" : "var(--energy)", maxWidth:340, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap" }}>
              {status}
            </span>
          )}
          {loading && <span style={{ color:"var(--accent)", animation:"pulse 1s infinite" }}>⟳</span>}
        </div>
        <WalletMultiButton style={{ fontFamily:"'Orbitron',sans-serif", fontSize:"0.62rem", background:"transparent", border:"1px solid var(--accent)", color:"var(--accent)", height:32, borderRadius:0 }} />
      </header>

      {/* ── Resource bar ── */}
      <div style={{ display:"flex", gap:20, padding:"6px 20px", background:"rgba(4,12,28,0.95)", borderBottom:"1px solid var(--border)", flexShrink:0, flexWrap:"wrap", alignItems:"center" }}>
        {[
          { label:"METAL",     val:displayRes.metal,     rate:resources?.metalHour||0,     color:"var(--metal)",   icon:"⬡" },
          { label:"CRYSTAL",   val:displayRes.crystal,   rate:resources?.crystalHour||0,   color:"var(--crystal)", icon:"◈" },
          { label:"DEUTERIUM", val:displayRes.deuterium, rate:resources?.deuteriumHour||0, color:"var(--deut)",    icon:"◉" },
        ].map(({ label, val, rate, color, icon }) => (
          <div key={label} style={{ display:"flex", alignItems:"center", gap:8 }}>
            <span style={{ fontSize:"1.1rem" }}>{icon}</span>
            <div>
              <div style={{ fontSize:"0.6rem", color:"var(--dim)", letterSpacing:"0.06em" }}>{label}</div>
              <div style={{ fontFamily:"'Share Tech Mono',monospace", fontSize:"0.82rem", color }}>{formatNum(val)}</div>
              {rate > 0 && <div style={{ fontSize:"0.6rem", color:"var(--accent3)" }}>+{formatNum(rate)}/h</div>}
            </div>
          </div>
        ))}
        {resources && (
          <>
            <div style={{ width:1, height:28, background:"var(--border)" }} />
            <div style={{ display:"flex", alignItems:"center", gap:8 }}>
              <span>⚡</span>
              <div>
                <div style={{ fontSize:"0.6rem", color:"var(--dim)" }}>ENERGY</div>
                <div style={{ fontFamily:"'Share Tech Mono',monospace", fontSize:"0.82rem", color: energy >= 0 ? "var(--energy)" : "var(--red)" }}>
                  {energy >= 0 ? "+" : ""}{formatNum(energy)}
                </div>
              </div>
            </div>
          </>
        )}
        {/* FIXED: Only ONE sync button */}
        {addresses && connected && (
          <button onClick={handleSettleProduction} disabled={loading}
            style={{ marginLeft:"auto", padding:"6px 16px", fontFamily:"'Orbitron',sans-serif", fontSize:"0.65rem", fontWeight:700, letterSpacing:"0.1em", background:"rgba(0,255,163,0.08)", border:"1px solid var(--accent3)", color:"var(--accent3)", cursor:"pointer" }}>
            {loading ? "⟳ SETTLING…" : "⛏ SYNC PRODUCTION"}
          </button>
        )}
        {!addresses && connected && !showNameInput && (
          <button onClick={() => setShowNameInput(true)}
            style={{ marginLeft:"auto", padding:"6px 16px", fontFamily:"'Orbitron',sans-serif", fontSize:"0.65rem", fontWeight:700, letterSpacing:"0.1em", background:"rgba(0,212,255,0.1)", border:"1px solid var(--accent)", color:"var(--accent)", cursor:"pointer" }}>
            ⚡ START IN SHARED WORLD
          </button>
        )}
        {showNameInput && (
          <div style={{ marginLeft:"auto", display:"flex", gap:8, alignItems:"center" }}>
            <input
              type="text" maxLength={19} value={planetName}
              onChange={e => setPlanetName(e.target.value)}
              placeholder="Planet name (max 19 chars)"
              style={{ padding:"5px 10px", fontSize:"0.72rem", width:200 }}
            />
            <button onClick={handleInit} disabled={loading}
              style={{ padding:"6px 14px", fontFamily:"'Orbitron',sans-serif", fontSize:"0.65rem", fontWeight:700, background:"rgba(0,212,255,0.1)", border:"1px solid var(--accent)", color:"var(--accent)", cursor:"pointer" }}>
              {loading ? "⟳ CREATING…" : "⚡ CREATE"}
            </button>
            <button onClick={() => setShowNameInput(false)}
              style={{ padding:"6px 10px", background:"transparent", border:"1px solid var(--border)", color:"var(--dim)", fontSize:"0.75rem", cursor:"pointer" }}>
              ✕
            </button>
          </div>
        )}
      </div>

      {/* ── Main layout ── */}
      <div style={{ display:"grid", gridTemplateColumns:"190px 1fr 260px", flex:1, overflow:"hidden" }}>

        {/* Sidebar */}
        <aside style={{ background:"rgba(4,10,24,0.97)", borderRight:"1px solid var(--border)", overflowY:"auto", padding:"10px 0" }}>
          {([
            { p:"overview"  as Page, icon:"🪐", label:"Overview" },
            { p:"buildings" as Page, icon:"🏗",  label:"Buildings" },
            { p:"shipyard"  as Page, icon:"🚀", label:"Shipyard" },
            { p:"fleet"     as Page, icon:"⚔️",  label:"Fleet", badge: fleet?.activeMissions || 0 },
            { p:"galaxy"    as Page, icon:"🌌", label:"Galaxy" },
            { p:"messages"  as Page, icon:"📡", label:"Messages" },
          ]).map(({ p, icon, label, badge }) => (
            <div key={p} onClick={() => setPage(p)}
              style={{ display:"flex", alignItems:"center", gap:10, padding:"9px 16px", cursor:"pointer",
                borderLeft:`2px solid ${page===p?"var(--accent)":"transparent"}`,
                background:page===p?"rgba(0,212,255,0.07)":"transparent",
                color:page===p?"var(--accent)":"var(--dim)", fontSize:"0.82rem", transition:"all 0.15s" }}>
              <span style={{ width:20, textAlign:"center" }}>{icon}</span>
              <span style={{ flex:1 }}>{label}</span>
              {badge ? <span style={{ fontSize:"0.58rem", padding:"1px 6px", background:"var(--accent2)", color:"#fff", fontFamily:"'Orbitron',sans-serif", borderRadius:2 }}>{badge}</span> : null}
            </div>
          ))}
        </aside>

        {/* Center */}
        <main style={{ overflowY:"auto", padding:20, background:"var(--bg)" }}>
          {!connected && <ConnectPrompt />}
          {connected && !addresses && !showNameInput && <InitPrompt onInit={() => setShowNameInput(true)} />}
          {connected && addresses && (
            <>
              {page === "overview"  && <OverviewPage planet={planet} />}
              {page === "buildings" && <BuildingsPage planet={planet} resources={resources} displayRes={displayRes} loading={loading} onBuild={handleBuild} onFinish={handleFinishBuild} />}
              {page === "shipyard"  && <ShipyardPage fleet={fleet} resources={resources} displayRes={displayRes} loading={loading} onBuildShip={handleBuildShip} />}
              {page === "fleet"     && <FleetPage fleet={fleet} />}
              {page === "galaxy"    && <GalaxyPage planet={planet} myOwner={publicKey ?? undefined} client={clientRef.current} />}
              {page === "messages"  && <MessagesPage />}
            </>
          )}
        </main>

        {/* Right panel */}
        <aside style={{ background:"rgba(4,10,24,0.97)", borderLeft:"1px solid var(--border)", overflowY:"auto", padding:16 }}>
          <div style={{ textAlign:"center", paddingBottom:14, borderBottom:"1px solid var(--border)", marginBottom:14 }}>
            <div style={{ width:76, height:76, borderRadius:"50%", margin:"0 auto 10px", background:"radial-gradient(circle at 35% 35%, #1a4a8a, #0a2040 50%, #050f20 80%)", boxShadow:"0 0 30px rgba(0,100,200,0.4)" }} />
            <div style={{ fontFamily:"'Orbitron',sans-serif", fontWeight:700, fontSize:"0.88rem", marginBottom:3 }}>{planet?.name || "—"}</div>
            <div style={{ fontFamily:"'Share Tech Mono',monospace", fontSize:"0.68rem", color:"var(--dim)" }}>
              {planet ? `[${planet.galaxy}:${planet.system}:${planet.position}]` : "No planet"}
            </div>
          </div>
          {planet && (
            <>
              {[
                ["Temperature", `${planet.temperature}°C`],
                ["Diameter",    `${planet.diameter.toLocaleString()} km`],
                ["Fields",      `${planet.usedFields} / ${planet.maxFields}`],
              ].map(([l, v]) => (
                <div key={l} style={{ display:"flex", justifyContent:"space-between", fontSize:"0.7rem", marginBottom:6 }}>
                  <span style={{ color:"var(--dim)" }}>{l}</span>
                  <span style={{ fontFamily:"'Share Tech Mono',monospace" }}>{v}</span>
                </div>
              ))}
              <div style={{ height:4, background:"rgba(255,255,255,0.05)", borderRadius:2, margin:"8px 0 14px", overflow:"hidden" }}>
                <div style={{ height:"100%", width:`${(planet.usedFields/Math.max(planet.maxFields,1))*100}%`, background:"linear-gradient(90deg,var(--accent3),var(--accent))" }} />
              </div>
            </>
          )}
          {addresses && (
            <div style={{ borderTop:"1px solid var(--border)", paddingTop:12 }}>
              <div style={{ fontFamily:"'Orbitron',sans-serif", fontSize:"0.57rem", letterSpacing:"0.12em", color:"var(--dim)", marginBottom:8 }}>ON-CHAIN</div>
              <div style={{ fontSize:"0.63rem", marginBottom:4 }}>Wallet: <span style={{ fontFamily:"'Share Tech Mono',monospace", fontSize:"0.58rem", color:"var(--accent)" }}>{publicKey?.toBase58().slice(0,16)}…</span></div>
              <div style={{ fontSize:"0.63rem", marginBottom:4 }}>Entity: <span style={{ fontFamily:"'Share Tech Mono',monospace", fontSize:"0.58rem", color:"var(--accent)" }}>{addresses.entityPda.toBase58().slice(0,16)}…</span></div>
              <div style={{ fontSize:"0.63rem" }}>Network: <span style={{ color:"var(--accent3)" }}>Devnet</span></div>
              <a href={`https://explorer.solana.com/address/${addresses.entityPda.toBase58()}?cluster=devnet`}
                target="_blank" rel="noreferrer"
                style={{ display:"block", marginTop:8, padding:"6px", textAlign:"center", fontFamily:"'Orbitron',sans-serif", fontSize:"0.58rem", background:"rgba(0,212,255,0.05)", border:"1px solid var(--border)", color:"var(--accent)", textDecoration:"none" }}>
                VIEW ON EXPLORER ↗
              </a>
            </div>
          )}
        </aside>
      </div>
    </div>
  );
}

// ── Pages ─────────────────────────────────────────────────────────────────────

function ConnectPrompt() {
  return (
    <div style={{ display:"flex", flexDirection:"column", alignItems:"center", justifyContent:"center", height:"100%", gap:16 }}>
      <div style={{ fontFamily:"'Orbitron',sans-serif", fontSize:"1.5rem", color:"var(--accent)" }}>SOLARGRID</div>
      <div style={{ color:"var(--dim)", fontSize:"0.85rem" }}>Connect your Solana wallet to play</div>
      <div style={{ fontSize:"0.75rem", color:"var(--dim)" }}>Switch wallet to <strong style={{color:"var(--accent3)"}}>Devnet</strong></div>
    </div>
  );
}

function InitPrompt({ onInit }: { onInit: () => void }) {
  return (
    <div style={{ display:"flex", flexDirection:"column", alignItems:"center", justifyContent:"center", height:"100%", gap:16 }}>
      <div style={{ fontFamily:"'Orbitron',sans-serif", fontSize:"1.2rem", color:"var(--accent)" }}>Initialize Your Empire</div>
      <div style={{ color:"var(--dim)", fontSize:"0.8rem", textAlign:"center", maxWidth:420, lineHeight:1.7 }}>
        Creates your BOLT ECS entity and attaches Planet, Resources and Fleet components in the shared world.
        Then runs <code style={{color:"var(--accent3)"}}>system-initialize</code> to seed real planet data.
        <br /><br />Cost: ~0.01 SOL for rent · Network: Devnet
      </div>
      <button onClick={onInit}
        style={{ padding:"12px 32px", fontFamily:"'Orbitron',sans-serif", fontSize:"0.8rem", fontWeight:700, letterSpacing:"0.15em", background:"rgba(0,212,255,0.1)", border:"1px solid var(--accent)", color:"var(--accent)", cursor:"pointer" }}>
        ⚡ CREATE PLANET
      </button>
    </div>
  );
}

function OverviewPage({ planet }: { planet: OnChainPlanet | null }) {
  if (!planet) return <div style={{color:"var(--dim)"}}>Loading planet data…</div>;
  return (
    <div>
      <PageTitle>🪐 Overview</PageTitle>
      <div style={{ display:"grid", gridTemplateColumns:"repeat(auto-fill,minmax(160px,1fr))", gap:12 }}>
        {BUILDINGS.map(b => {
          const lvl = (planet as any)[b.key] || 0;
          return (
            <div key={b.key} style={{ background:"var(--panel)", border:"1px solid var(--border)", padding:14, clipPath:"polygon(0 0,calc(100% - 10px) 0,100% 10px,100% 100%,0 100%)" }}>
              <div style={{ display:"flex", justifyContent:"space-between", marginBottom:6 }}>
                <span style={{ fontSize:"1.2rem" }}>{b.icon}</span>
                <span style={{ fontFamily:"'Orbitron',sans-serif", fontWeight:900, fontSize:"1.1rem", color:lvl>0?"var(--accent)":"var(--dim)" }}>{lvl}</span>
              </div>
              <div style={{ fontSize:"0.74rem", fontWeight:600 }}>{b.name}</div>
              <div style={{ fontSize:"0.62rem", color:"var(--dim)", marginTop:3 }}>{b.desc}</div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function BuildingsPage({ planet, resources, displayRes, loading, onBuild, onFinish }: {
  planet: OnChainPlanet | null;
  resources: OnChainResources | null;
  displayRes: { metal:number; crystal:number; deuterium:number };
  loading: boolean;
  onBuild: (idx:number, name:string) => void;
  onFinish: () => void;
}) {
  const [, setTick] = useState(0);
  useEffect(() => { const id = setInterval(() => setTick(t => t+1), 1000); return () => clearInterval(id); }, []);
  const now = Math.floor(Date.now()/1000);
  if (!planet) return <div style={{color:"var(--dim)"}}>Loading…</div>;

  return (
    <div>
      <PageTitle>🏗 Buildings</PageTitle>

      {planet.buildFinishTs > 0 && (
        <div style={{ padding:"10px 14px", background:"rgba(255,107,26,0.07)", border:"1px solid var(--accent2)", borderLeft:"3px solid var(--accent2)", marginBottom:16, display:"flex", justifyContent:"space-between", alignItems:"center" }}>
          <span style={{ fontSize:"0.75rem" }}>⚙ Upgrading <strong style={{color:"var(--accent2)"}}>{BUILDINGS.find(b=>b.idx===planet.buildQueueItem)?.name}</strong> → Lv.{planet.buildQueueTarget}</span>
          <div style={{ display:"flex", alignItems:"center", gap:8 }}>
            <span style={{ fontFamily:"'Share Tech Mono',monospace", color:"var(--accent3)", fontSize:"0.75rem" }}>{formatDuration(Math.max(0, planet.buildFinishTs - now))}</span>
            {now >= planet.buildFinishTs && (
              <button onClick={onFinish} disabled={loading}
                style={{ padding:"4px 12px", fontFamily:"'Orbitron',sans-serif", fontSize:"0.6rem", background:"rgba(57,255,20,0.1)", border:"1px solid var(--accent3)", color:"var(--accent3)", cursor:"pointer" }}>
                COLLECT ✓
              </button>
            )}
          </div>
        </div>
      )}

      <div style={{ display:"grid", gridTemplateColumns:"repeat(auto-fill,minmax(200px,1fr))", gap:12 }}>
        {BUILDINGS.map(b => {
          const level     = (planet as any)[b.key] || 0;
          const cost      = buildCost(b.idx, level);
          const canAfford = displayRes.metal >= cost.m && displayRes.crystal >= cost.c && displayRes.deuterium >= cost.d;
          const busy      = planet.buildFinishTs > 0;
          const isBuilding = busy && planet.buildQueueItem === b.idx;
          const dur       = buildSeconds(b.idx, level+1, planet.roboticsFactory);
          const pct       = isBuilding
            ? Math.min(100, Math.max(0, ((now - (planet.buildFinishTs - dur)) / dur) * 100))
            : 0;

          return (
            <div key={b.key} style={{ background:"var(--panel)", border:`1px solid ${isBuilding?"var(--accent2)":"var(--border)"}`, padding:14, clipPath:"polygon(0 0,calc(100% - 10px) 0,100% 10px,100% 100%,0 100%)", position:"relative" }}>
              {isBuilding && <div style={{ position:"absolute", top:6, right:8, fontSize:"0.5rem", fontFamily:"'Orbitron',sans-serif", color:"var(--accent2)" }}>UPGRADING</div>}
              <div style={{ display:"flex", justifyContent:"space-between", alignItems:"flex-start", marginBottom:8 }}>
                <div>
                  <div style={{ fontSize:"0.77rem", fontWeight:600 }}>{b.icon} {b.name}</div>
                  <div style={{ fontSize:"0.62rem", color:"var(--dim)", marginTop:2 }}>{b.desc}</div>
                </div>
                <span style={{ fontFamily:"'Orbitron',sans-serif", fontWeight:900, fontSize:"1.1rem", color:level>0?"var(--accent)":"var(--dim)", flexShrink:0 }}>{level}</span>
              </div>
              <div style={{ display:"flex", gap:4, flexWrap:"wrap", marginBottom:6 }}>
                {cost.m>0 && <CostPill label="⬡" val={cost.m} ok={displayRes.metal>=cost.m} />}
                {cost.c>0 && <CostPill label="◈" val={cost.c} ok={displayRes.crystal>=cost.c} />}
                {cost.d>0 && <CostPill label="◉" val={cost.d} ok={displayRes.deuterium>=cost.d} />}
              </div>
              <div style={{ fontSize:"0.62rem", color:"var(--dim)", marginBottom:8 }}>⏱ {formatDuration(dur)}</div>
              {isBuilding ? (
                <div>
                  <div style={{ height:3, background:"rgba(0,212,255,0.1)", overflow:"hidden" }}>
                    <div style={{ height:"100%", width:`${pct}%`, background:"linear-gradient(90deg,var(--accent),var(--accent2))", transition:"width 1s linear" }} />
                  </div>
                  <div style={{ fontSize:"0.62rem", color:"var(--accent2)", marginTop:4, fontFamily:"'Share Tech Mono',monospace" }}>
                    {formatDuration(Math.max(0, planet.buildFinishTs - now))}
                  </div>
                </div>
              ) : (
                <button disabled={!canAfford || busy || loading} onClick={() => onBuild(b.idx, b.name)}
                  style={{ width:"100%", padding:7, fontFamily:"'Orbitron',sans-serif", fontSize:"0.62rem", fontWeight:700, letterSpacing:"0.1em", background:"transparent", border:`1px solid ${canAfford&&!busy?"var(--accent)":"var(--border)"}`, color:canAfford&&!busy?"var(--accent)":"var(--dim)", cursor:canAfford&&!busy&&!loading?"pointer":"not-allowed" }}>
                  ▲ UPGRADE → LV.{level+1}
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ShipyardPage({ fleet, resources, displayRes, loading, onBuildShip }: {
  fleet: OnChainFleet | null;
  resources: OnChainResources | null;
  displayRes: { metal:number; crystal:number; deuterium:number };
  loading: boolean;
  onBuildShip: (key:string, name:string, qty:number) => void;
}) {
  const [qtys, setQtys] = useState<Record<string,number>>({});
  return (
    <div>
      <PageTitle>🚀 Shipyard</PageTitle>
      <div style={{ display:"grid", gridTemplateColumns:"repeat(auto-fill,minmax(165px,1fr))", gap:12 }}>
        {SHIPS.map(s => {
          const qty        = qtys[s.key] || 1;
          const stationed  = fleet ? (fleet as any)[s.key] || 0 : 0;
          const totalM     = s.cost.m * qty;
          const totalC     = s.cost.c * qty;
          const totalD     = s.cost.d * qty;
          const canAfford  = displayRes.metal >= totalM && displayRes.crystal >= totalC && displayRes.deuterium >= totalD;
          return (
            <div key={s.key} style={{ background:"var(--panel)", border:"1px solid var(--border)", padding:14, textAlign:"center", clipPath:"polygon(0 0,calc(100% - 10px) 0,100% 10px,100% 100%,0 100%)" }}>
              <div style={{ fontSize:"1.8rem", marginBottom:4 }}>{s.icon}</div>
              <div style={{ fontSize:"0.72rem", fontWeight:600, marginBottom:2 }}>{s.name}</div>
              <div style={{ fontFamily:"'Orbitron',sans-serif", fontWeight:900, fontSize:"1rem", color:stationed>0?"var(--text)":"var(--dim)", marginBottom:6 }}>
                {stationed > 0 ? stationed.toLocaleString() : "0"}
              </div>
              <div style={{ fontSize:"0.6rem", color:"var(--dim)", marginBottom:8 }}>⚔ {s.atk.toLocaleString()} · 📦 {s.cargo.toLocaleString()}</div>
              <div style={{ display:"flex", gap:4, flexWrap:"wrap", justifyContent:"center", marginBottom:8 }}>
                {s.cost.m>0 && <CostPill label="⬡" val={totalM} ok={displayRes.metal>=totalM} />}
                {s.cost.c>0 && <CostPill label="◈" val={totalC} ok={displayRes.crystal>=totalC} />}
                {s.cost.d>0 && <CostPill label="◉" val={totalD} ok={displayRes.deuterium>=totalD} />}
              </div>
              <div style={{ display:"flex", gap:4 }}>
                <input type="number" min={1} value={qty}
                  onChange={e => setQtys(q => ({...q, [s.key]: Math.max(1, parseInt(e.target.value)||1)}))}
                  style={{ width:50, padding:"5px 4px", fontSize:"0.78rem", textAlign:"center" }} />
                <button disabled={!canAfford || loading} onClick={() => onBuildShip(s.key, s.name, qty)}
                  style={{ flex:1, padding:"5px 0", fontFamily:"'Orbitron',sans-serif", fontSize:"0.58rem", fontWeight:700, background:"transparent", border:`1px solid ${canAfford&&!loading?"var(--accent)":"var(--border)"}`, color:canAfford&&!loading?"var(--accent)":"var(--dim)", cursor:canAfford&&!loading?"pointer":"not-allowed" }}>
                  BUILD
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function FleetPage({ fleet }: { fleet: OnChainFleet | null }) {
  const [, setTick] = useState(0);
  useEffect(() => { const id = setInterval(() => setTick(t => t+1), 1000); return () => clearInterval(id); }, []);
  const now = Math.floor(Date.now()/1000);
  const mLabels: Record<number,string> = {1:"ATTACK",2:"TRANSPORT",3:"DEPLOY",4:"ESPIONAGE",5:"COLONIZE",6:"RECYCLE"};
  const mColors: Record<number,string> = {1:"var(--red)",2:"var(--accent)",3:"var(--accent3)",4:"var(--energy)"};

  return (
    <div>
      <PageTitle>⚔️ Fleet</PageTitle>
      <SectionTitle>Stationed Ships</SectionTitle>
      <div style={{ display:"grid", gridTemplateColumns:"repeat(auto-fill,minmax(120px,1fr))", gap:8, marginBottom:24 }}>
        {SHIPS.map(s => {
          const count = fleet ? (fleet as any)[s.key] || 0 : 0;
          return (
            <div key={s.key} style={{ background:"var(--panel)", border:"1px solid var(--border)", padding:10, textAlign:"center" }}>
              <div style={{ fontSize:"1.4rem" }}>{s.icon}</div>
              <div style={{ fontSize:"0.62rem", color:"var(--dim)", margin:"3px 0 2px" }}>{s.name}</div>
              <div style={{ fontFamily:"'Orbitron',sans-serif", fontSize:count>0?"0.95rem":"0.75rem", fontWeight:700, color:count>0?"var(--text)":"var(--dim)" }}>
                {count > 0 ? count.toLocaleString() : "—"}
              </div>
            </div>
          );
        })}
      </div>

      {fleet && fleet.missions.length > 0 && (
        <>
          <SectionTitle>Active Missions ({fleet.activeMissions})</SectionTitle>
          <table style={{ width:"100%", borderCollapse:"collapse", fontSize:"0.73rem" }}>
            <thead>
              <tr style={{ borderBottom:"1px solid var(--border)" }}>
                {["TYPE","STATUS","ETA"].map(h => (
                  <th key={h} style={{ padding:"5px 8px", textAlign:"left", fontFamily:"'Orbitron',sans-serif", fontSize:"0.58rem", letterSpacing:"0.1em", color:"var(--dim)" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {fleet.missions.map((m, i) => {
                const inFlight  = !m.applied;
                const returning = m.applied && m.returnTs > 0 && now < m.returnTs;
                const done      = m.applied && now >= m.returnTs;
                const eta       = inFlight  ? Math.max(0, m.arriveTs - now)
                                : returning ? Math.max(0, m.returnTs - now)
                                : 0;
                return (
                  <tr key={i} style={{ borderBottom:"1px solid rgba(255,255,255,0.04)" }}>
                    <td style={{ padding:"7px 8px" }}>
                      <span style={{ padding:"2px 7px", fontFamily:"'Orbitron',sans-serif", fontSize:"0.57rem", fontWeight:700, border:"1px solid", color:mColors[m.missionType]||"var(--dim)", borderColor:mColors[m.missionType]||"var(--border)" }}>
                        {mLabels[m.missionType]||"?"}
                      </span>
                    </td>
                    <td style={{ padding:"7px 8px", fontSize:"0.68rem", color:"var(--dim)" }}>
                      {done ? "✅ RETURNED" : returning ? "↩ RETURNING" : "→ IN FLIGHT"}
                    </td>
                    <td style={{ padding:"7px 8px", fontFamily:"'Share Tech Mono',monospace", color:done?"var(--accent3)":"var(--accent3)", fontSize:"0.72rem" }}>
                      {done ? "—" : formatDuration(eta)}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </>
      )}
      {(!fleet || fleet.missions.length === 0) && (
        <div style={{ color:"var(--dim)", fontSize:"0.8rem", padding:"20px 0" }}>No active missions</div>
      )}
    </div>
  );
}

/** FIXED: Galaxy page now fetches REAL on-chain planet accounts */
function GalaxyPage({ planet, myOwner, client }: {
  planet: OnChainPlanet | null;
  myOwner?: PublicKey;
  client: SolarGridClient | null;
}) {
  const [galaxy, setGalaxy]   = useState(planet?.galaxy || 1);
  const [system, setSystem]   = useState(planet?.system || 1);
  const [entries, setEntries] = useState<GalaxyEntry[]>([]);
  const [scanning, setScanning] = useState(false);

  const doScan = useCallback(async () => {
    if (!client) return;
    setScanning(true);
    try {
      const res = await client.scanGalaxy(galaxy, system, myOwner);
      setEntries(res);
    } catch (e) {
      console.error("scanGalaxy:", e);
    }
    setScanning(false);
  }, [client, galaxy, system, myOwner]);

  // Auto-scan when galaxy/system changes or planet first loads
  useEffect(() => { doScan(); }, [galaxy, system]);

  // Build a position → entry map for quick lookup
  const entryMap = new Map<number, GalaxyEntry>();
  for (const e of entries) entryMap.set(e.position, e);

  return (
    <div>
      <PageTitle>🌌 Galaxy</PageTitle>
      <div style={{ display:"flex", gap:10, alignItems:"center", marginBottom:16, flexWrap:"wrap" }}>
        <span style={{ fontSize:"0.7rem", color:"var(--dim)" }}>Galaxy</span>
        <input type="number" min={1} max={9}   value={galaxy} onChange={e => setGalaxy(Math.max(1,Math.min(9,+e.target.value)))}   style={{ width:60, padding:"5px 8px", fontSize:"0.8rem" }} />
        <span style={{ fontSize:"0.7rem", color:"var(--dim)" }}>System</span>
        <input type="number" min={1} max={499} value={system} onChange={e => setSystem(Math.max(1,Math.min(499,+e.target.value)))} style={{ width:70, padding:"5px 8px", fontSize:"0.8rem" }} />
        <button onClick={doScan} disabled={scanning}
          style={{ padding:"5px 14px", fontFamily:"'Orbitron',sans-serif", fontSize:"0.62rem", background:"rgba(0,212,255,0.08)", border:"1px solid var(--accent)", color:"var(--accent)", cursor:"pointer" }}>
          {scanning ? "⟳ SCANNING…" : "SCAN"}
        </button>
        <span style={{ fontSize:"0.65rem", color:"var(--dim)" }}>
          {entries.length === 0 && !scanning ? "Empty system — no planets found" : `${entries.length} planet(s) found`}
        </span>
      </div>

      <table style={{ width:"100%", borderCollapse:"collapse", fontSize:"0.73rem" }}>
        <thead>
          <tr style={{ borderBottom:"1px solid var(--border)" }}>
            {["POS","PLANET","OWNER","ACTIONS"].map(h => (
              <th key={h} style={{ padding:"6px 8px", textAlign:"left", fontFamily:"'Orbitron',sans-serif", fontSize:"0.57rem", letterSpacing:"0.1em", color:"var(--dim)" }}>{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {Array.from({length:15},(_,i)=>i+1).map(pos => {
            const e     = entryMap.get(pos);
            const isMe  = e?.isMe || false;
            return (
              <tr key={pos} style={{ borderBottom:"1px solid rgba(255,255,255,0.04)", background:isMe?"rgba(0,212,255,0.04)":"transparent" }}>
                <td style={{ padding:"8px", color:"var(--dim)", fontFamily:"'Share Tech Mono',monospace" }}>{pos}</td>
                <td style={{ padding:"8px", color:isMe?"var(--accent3)":e?"var(--text)":"var(--dim)" }}>
                  {e ? `${e.name}${isMe?" ★":""}` : "—"}
                </td>
                <td style={{ padding:"8px", fontFamily:"'Share Tech Mono',monospace", fontSize:"0.62rem", color:isMe?"var(--accent3)":"var(--crystal)" }}>
                  {e ? (isMe ? "You" : `${e.owner.slice(0,8)}…`) : "—"}
                </td>
                <td style={{ padding:"8px" }}>
                  {e && !isMe && (
                    <div style={{ display:"flex", gap:4 }}>
                      <button style={{ padding:"3px 8px", background:"rgba(255,51,85,0.1)", border:"1px solid rgba(255,51,85,0.4)", color:"var(--red)", fontSize:"0.62rem", cursor:"pointer" }}>⚔ ATK</button>
                      <button style={{ padding:"3px 8px", background:"rgba(0,212,255,0.05)", border:"1px solid var(--border)", color:"var(--dim)", fontSize:"0.62rem", cursor:"pointer" }}>👁 SPY</button>
                    </div>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function MessagesPage() {
  return (
    <div>
      <PageTitle>📡 Messages</PageTitle>
      <div style={{ color:"var(--dim)", fontSize:"0.8rem", padding:"20px 0" }}>
        No messages yet. Messages appear here when fleets arrive, battles resolve, or builds complete.
      </div>
    </div>
  );
}

// ── Shared UI atoms ────────────────────────────────────────────────────────────

function PageTitle({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ fontFamily:"'Orbitron',sans-serif", fontWeight:700, fontSize:"1rem", color:"var(--accent)", letterSpacing:"0.1em", marginBottom:18, display:"flex", alignItems:"center", gap:12 }}>
      {children}
      <div style={{ flex:1, height:1, background:"linear-gradient(90deg,var(--bhi),transparent)" }} />
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ fontFamily:"'Orbitron',sans-serif", fontSize:"0.65rem", fontWeight:600, letterSpacing:"0.12em", color:"var(--dim)", marginBottom:10, paddingBottom:6, borderBottom:"1px solid var(--border)" }}>
      {children}
    </div>
  );
}

function CostPill({ label, val, ok }: { label:string; val:number; ok:boolean }) {
  return (
    <span style={{ display:"inline-flex", alignItems:"center", gap:2, fontFamily:"'Share Tech Mono',monospace", fontSize:"0.6rem", padding:"2px 5px", background:"rgba(0,0,0,0.3)", border:`1px solid ${ok?"rgba(255,255,255,0.06)":"rgba(255,51,85,0.35)"}`, color:ok?"var(--text)":"var(--red)", borderRadius:2 }}>
      {label} {formatNum(val)}
    </span>
  );
}
