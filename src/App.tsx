import React, { useEffect, useState, useCallback, useRef } from "react";
import { useConnection, useWallet, useAnchorWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { AnchorProvider } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

import {
  GameClient,
  Planet, Resources, Fleet, Mission, PlayerState,
  BUILDINGS, SHIPS, SHIP_TYPE_IDX, MISSION_LABELS,
  upgradeCost, buildTimeSecs,
  fmt, fmtCountdown, missionProgress, energyEfficiency,
} from "./game";

// ─── Types ────────────────────────────────────────────────────────────────────
type Tab = "overview" | "buildings" | "shipyard" | "fleet" | "missions";

// ─── CSS ──────────────────────────────────────────────────────────────────────
const CSS = `
  @import url('https://fonts.googleapis.com/css2?family=Orbitron:wght@400;600;700;900&family=Share+Tech+Mono&display=swap');

  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

  :root {
    --void:    #04040d;
    --panel:   #0b0b1e;
    --border:  #1a1a3a;
    --purple:  #9b5de5;
    --cyan:    #00f5d4;
    --text:    #c8d6e5;
    --dim:     #4a5568;
    --metal:   #b8b8d4;
    --crystal: #00f5d4;
    --deut:    #4cc9f0;
    --danger:  #ff006e;
    --success: #06d6a0;
    --warn:    #ffd60a;
    --glow-p:  0 0 20px rgba(155,93,229,0.4);
    --glow-c:  0 0 20px rgba(0,245,212,0.4);
  }

  html, body, #root { height: 100%; background: var(--void); color: var(--text);
    font-family: 'Share Tech Mono', monospace; font-size: 13px; overflow: hidden; }

  .starfield { position: fixed; inset: 0; z-index: 0; overflow: hidden; pointer-events: none; }
  .star { position: absolute; border-radius: 50%; background: white;
    animation: twinkle var(--dur) ease-in-out infinite; animation-delay: var(--delay); }
  @keyframes twinkle { 0%,100%{opacity:var(--min-op);transform:scale(1)} 50%{opacity:1;transform:scale(1.4)} }

  .app { position: relative; z-index: 1; height: 100vh; display: grid;
    grid-template-rows: 56px 1fr; grid-template-columns: 220px 1fr;
    grid-template-areas: "header header" "sidebar main"; }

  .header { grid-area: header; display: flex; align-items: center; justify-content: space-between;
    padding: 0 24px; background: rgba(8,8,22,0.95); border-bottom: 1px solid var(--border);
    backdrop-filter: blur(12px); }
  .logo-area { display: flex; align-items: center; gap: 12px; }
  .game-title { font-family: 'Orbitron', sans-serif; font-size: 16px; font-weight: 900;
    letter-spacing: 3px; background: linear-gradient(135deg, var(--purple), var(--cyan));
    -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
  .header-right { display: flex; align-items: center; gap: 12px; }
  .chain-tag { font-size: 10px; letter-spacing: 1px; color: var(--dim);
    border: 1px solid var(--border); padding: 4px 8px; border-radius: 2px; }

  .sidebar { grid-area: sidebar; background: rgba(11,11,30,0.9);
    border-right: 1px solid var(--border); display: flex; flex-direction: column; overflow: hidden; }
  .planet-card { padding: 20px 16px; border-bottom: 1px solid var(--border); }
  .planet-coords { font-size: 10px; color: var(--dim); letter-spacing: 1px; margin-bottom: 6px; }
  .planet-name { font-family: 'Orbitron', sans-serif; font-size: 14px; font-weight: 700; color: white; margin-bottom: 2px; }
  .planet-meta { font-size: 10px; color: var(--dim); }
  .fields-bar { margin-top: 10px; height: 3px; background: var(--border); border-radius: 2px; overflow: hidden; }
  .fields-fill { height: 100%; background: linear-gradient(90deg, var(--purple), var(--cyan)); transition: width 0.5s; }
  .fields-label { margin-top: 4px; font-size: 10px; color: var(--dim); display: flex; justify-content: space-between; }

  .res-panel { padding: 14px 16px; border-bottom: 1px solid var(--border); flex-shrink: 0; }
  .res-label { font-size: 9px; letter-spacing: 2px; color: var(--dim); text-transform: uppercase; margin-bottom: 10px; }
  .res-row { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  .res-name { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--dim); }
  .res-dot { width: 6px; height: 6px; border-radius: 50%; }
  .res-val { font-size: 12px; font-weight: 600; }
  .res-rate { font-size: 9px; color: var(--dim); }
  .cap-bar { margin-bottom: 12px; height: 2px; background: var(--border); border-radius: 1px; overflow: hidden; }
  .cap-fill { height: 100%; border-radius: 1px; transition: width 0.5s; }
  .energy-row { display: flex; align-items: center; justify-content: space-between;
    padding: 8px 0; border-top: 1px solid var(--border); }

  .nav { flex: 1; padding: 12px 0; overflow-y: auto; }
  .nav-item { display: flex; align-items: center; gap: 10px; padding: 10px 16px; cursor: pointer;
    font-size: 11px; letter-spacing: 1.5px; text-transform: uppercase; color: var(--dim);
    transition: all 0.15s; border-left: 2px solid transparent; }
  .nav-item:hover { color: var(--text); background: rgba(155,93,229,0.05); }
  .nav-item.active { color: var(--cyan); border-left-color: var(--cyan); background: rgba(0,245,212,0.05); }
  .nav-badge { margin-left: auto; font-size: 9px; padding: 2px 6px;
    background: var(--danger); border-radius: 10px; color: white; font-weight: 700; }

  .main { grid-area: main; overflow-y: auto; padding: 24px;
    scrollbar-width: thin; scrollbar-color: var(--border) transparent; }
  .main::-webkit-scrollbar { width: 4px; }
  .main::-webkit-scrollbar-thumb { background: var(--border); }

  .section-title { font-family: 'Orbitron', sans-serif; font-size: 12px; font-weight: 700;
    letter-spacing: 3px; color: var(--purple); text-transform: uppercase; margin-bottom: 20px;
    padding-bottom: 8px; border-bottom: 1px solid var(--border);
    display: flex; align-items: center; gap: 10px; }
  .section-title::after { content:''; flex:1; height:1px; background: linear-gradient(90deg, var(--border), transparent); }

  .grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .grid-3 { display: grid; grid-template-columns: repeat(3,1fr); gap: 12px; }
  .grid-4 { display: grid; grid-template-columns: repeat(4,1fr); gap: 10px; }

  .card { background: var(--panel); border: 1px solid var(--border); border-radius: 4px; padding: 16px; transition: border-color 0.2s; }
  .card:hover { border-color: rgba(155,93,229,0.3); }
  .card-label { font-size: 9px; letter-spacing: 2px; color: var(--dim); text-transform: uppercase; margin-bottom: 6px; }
  .card-value { font-family: 'Orbitron', sans-serif; font-size: 20px; font-weight: 700; color: white; }
  .card-sub { font-size: 10px; color: var(--dim); margin-top: 3px; }

  .building-card { background: var(--panel); border: 1px solid var(--border); border-radius: 4px;
    padding: 14px; display: flex; flex-direction: column; gap: 8px; transition: all 0.2s; }
  .building-card:hover { border-color: rgba(155,93,229,0.4); }
  .building-header { display: flex; align-items: center; justify-content: space-between; }
  .building-icon-name { display: flex; align-items: center; gap: 8px; }
  .building-icon { font-size: 16px; }
  .building-name { font-size: 11px; color: var(--text); }
  .building-level { font-family: 'Orbitron', sans-serif; font-size: 16px; font-weight: 700; color: var(--purple); }
  .building-costs { font-size: 10px; color: var(--dim); display: flex; flex-direction: column; gap: 2px; }
  .building-cost-row { display: flex; justify-content: space-between; }
  .cost-ok { color: var(--text); } .cost-bad { color: var(--danger); }
  .build-btn { font-family: 'Share Tech Mono', monospace; font-size: 10px; letter-spacing: 1px;
    padding: 6px 10px; border-radius: 2px; border: none; cursor: pointer; transition: all 0.15s;
    text-transform: uppercase; width: 100%; }
  .build-btn.can-build { background: linear-gradient(135deg,rgba(155,93,229,0.2),rgba(0,245,212,0.1));
    border: 1px solid var(--purple); color: var(--purple); }
  .build-btn.can-build:hover { background: linear-gradient(135deg,var(--purple),var(--cyan));
    color: var(--void); box-shadow: var(--glow-p); }
  .build-btn.building-now { background: rgba(255,214,10,0.1); border: 1px solid var(--warn);
    color: var(--warn); cursor: default; }
  .build-btn.finish-btn { background: rgba(6,214,160,0.1); border: 1px solid var(--success); color: var(--success); }
  .build-btn.finish-btn:hover { background: var(--success); color: var(--void); }
  .build-btn.no-funds { background: transparent; border: 1px solid var(--border); color: var(--dim); cursor: not-allowed; }

  /* Shipyard cards */
  .ship-build-card { background: var(--panel); border: 1px solid var(--border); border-radius: 4px;
    padding: 14px; display: flex; flex-direction: column; gap: 8px; transition: border-color 0.2s; }
  .ship-build-card:hover { border-color: rgba(0,245,212,0.3); }
  .ship-build-header { display: flex; align-items: center; justify-content: space-between; }
  .ship-build-icon-name { display: flex; align-items: center; gap: 8px; }
  .ship-build-icon { font-size: 20px; }
  .ship-build-name { font-size: 11px; color: var(--text); }
  .ship-build-count { font-family: 'Orbitron', sans-serif; font-size: 14px; font-weight: 700; color: var(--cyan); }
  .ship-build-count.zero { color: var(--border); }
  .ship-build-stats { font-size: 9px; color: var(--dim); display: flex; gap: 10px; }
  .ship-qty-row { display: flex; align-items: center; gap: 6px; }
  .qty-input { width: 60px; padding: 4px 6px; font-size: 11px; border-radius: 2px; text-align: center; }
  .ship-build-btn { font-family: 'Share Tech Mono', monospace; font-size: 10px; letter-spacing: 1px;
    padding: 6px 10px; border-radius: 2px; border: 1px solid var(--cyan);
    background: rgba(0,245,212,0.08); color: var(--cyan); cursor: pointer; transition: all 0.15s;
    text-transform: uppercase; flex: 1; }
  .ship-build-btn:hover:not(:disabled) { background: var(--cyan); color: var(--void); box-shadow: var(--glow-c); }
  .ship-build-btn:disabled { border-color: var(--border); color: var(--dim); cursor: not-allowed; background: transparent; }

  /* Fleet hangar cards */
  .ship-card { background: var(--panel); border: 1px solid var(--border); border-radius: 4px;
    padding: 14px; display: flex; flex-direction: column; align-items: center; gap: 6px;
    transition: border-color 0.2s; position: relative; }
  .ship-card:hover { border-color: rgba(0,245,212,0.3); }
  .ship-icon { font-size: 22px; }
  .ship-name { font-size: 9px; color: var(--dim); text-align: center; letter-spacing: 1px; }
  .ship-count { font-family: 'Orbitron', sans-serif; font-size: 18px; font-weight: 700; color: var(--cyan); }
  .ship-count.zero { color: var(--border); }
  .launch-btn { font-size: 9px; letter-spacing: 1px; padding: 3px 8px; border-radius: 2px;
    border: 1px solid var(--purple); background: rgba(155,93,229,0.08); color: var(--purple);
    cursor: pointer; transition: all 0.15s; margin-top: 2px; }
  .launch-btn:hover { background: var(--purple); color: var(--void); }

  /* Mission cards */
  .mission-card { background: var(--panel); border: 1px solid var(--border); border-radius: 4px; padding: 16px; margin-bottom: 12px; }
  .mission-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
  .mission-type-badge { font-size: 10px; letter-spacing: 2px; padding: 3px 8px; border-radius: 2px; font-weight: 700; }
  .mission-type-badge.attack    { background:rgba(255,0,110,0.15); color:var(--danger); border:1px solid rgba(255,0,110,0.3); }
  .mission-type-badge.transport { background:rgba(0,245,212,0.1);  color:var(--cyan);   border:1px solid rgba(0,245,212,0.3); }
  .mission-type-badge.other     { background:rgba(155,93,229,0.1); color:var(--purple); border:1px solid rgba(155,93,229,0.3); }
  .mission-returning { font-size: 10px; color: var(--success); letter-spacing: 1px; }
  .progress-bar { height: 3px; background: var(--border); border-radius: 2px; overflow: hidden; margin-bottom: 8px; }
  .progress-fill { height: 100%; border-radius: 2px; transition: width 1s linear; }
  .progress-fill.outbound  { background: linear-gradient(90deg, var(--purple), var(--cyan)); }
  .progress-fill.returning { background: linear-gradient(90deg, var(--cyan), var(--success)); }
  .mission-info { display: flex; justify-content: space-between; font-size: 10px; color: var(--dim); }
  .mission-eta { color: var(--cyan); font-weight: 600; }
  .mission-ships { margin-top: 10px; display: flex; flex-wrap: wrap; gap: 8px; }
  .mission-ship-badge { font-size: 10px; background: rgba(155,93,229,0.08);
    border: 1px solid var(--border); border-radius: 2px; padding: 2px 6px; color: var(--text); }
  .apply-btn { font-family: 'Share Tech Mono', monospace; font-size: 10px; letter-spacing: 1px;
    padding: 6px 14px; border-radius: 2px; border: 1px solid var(--success);
    background: rgba(6,214,160,0.1); color: var(--success); cursor: pointer;
    transition: all 0.15s; margin-top: 10px; }
  .apply-btn:hover:not(:disabled) { background: var(--success); color: var(--void); }
  .apply-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .apply-btn.danger { border-color: var(--danger); background: rgba(255,0,110,0.08); color: var(--danger); }
  .apply-btn.danger:hover:not(:disabled) { background: var(--danger); color: var(--void); }

  .stat-row { display: flex; align-items: center; justify-content: space-between;
    padding: 10px 0; border-bottom: 1px solid rgba(26,26,58,0.5); }
  .stat-row:last-child { border-bottom: none; }
  .stat-key { color: var(--dim); font-size: 11px; letter-spacing: 1px; }
  .stat-val { font-size: 11px; color: var(--text); }

  .build-queue-banner { background: rgba(255,214,10,0.05); border: 1px solid rgba(255,214,10,0.2);
    border-radius: 4px; padding: 12px 16px; margin-bottom: 20px;
    display: flex; align-items: center; justify-content: space-between; }
  .build-queue-label { font-size: 10px; color: var(--warn); letter-spacing: 2px; text-transform: uppercase; }
  .build-queue-item-name { font-size: 13px; color: var(--text); margin-top: 2px; }
  .build-queue-right { text-align: right; }
  .build-queue-eta { font-family: 'Orbitron', sans-serif; font-size: 16px; font-weight: 700; color: var(--warn); }

  /* Launch Fleet Modal */
  .modal-backdrop { position: fixed; inset: 0; background: rgba(4,4,13,0.85);
    z-index: 100; display: flex; align-items: center; justify-content: center;
    backdrop-filter: blur(4px); }
  .modal { background: var(--panel); border: 1px solid var(--border); border-radius: 6px;
    padding: 28px; width: 560px; max-height: 85vh; overflow-y: auto;
    scrollbar-width: thin; scrollbar-color: var(--border) transparent; }
  .modal-title { font-family: 'Orbitron', sans-serif; font-size: 13px; font-weight: 700;
    letter-spacing: 3px; color: var(--cyan); margin-bottom: 20px;
    padding-bottom: 10px; border-bottom: 1px solid var(--border); }
  .modal-section { margin-bottom: 18px; }
  .modal-label { font-size: 9px; letter-spacing: 2px; color: var(--dim);
    text-transform: uppercase; margin-bottom: 10px; }
  .modal-ship-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; margin-bottom: 12px; }
  .modal-ship-row { display: flex; align-items: center; justify-content: space-between;
    background: rgba(0,0,0,0.3); border: 1px solid var(--border); border-radius: 3px; padding: 6px 8px; }
  .modal-ship-label { font-size: 10px; color: var(--text); display: flex; align-items: center; gap: 5px; }
  .modal-ship-avail { font-size: 9px; color: var(--dim); }
  .modal-input { width: 64px; padding: 4px 6px; font-size: 11px; border-radius: 2px; text-align: right; }
  .modal-select { padding: 6px 10px; font-size: 11px; border-radius: 2px; width: 100%; }
  .modal-row { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  .modal-footer { display: flex; gap: 10px; margin-top: 20px; padding-top: 16px; border-top: 1px solid var(--border); }
  .modal-btn { font-family: 'Share Tech Mono', monospace; font-size: 11px; letter-spacing: 1px;
    padding: 9px 18px; border-radius: 2px; cursor: pointer; transition: all 0.15s;
    text-transform: uppercase; flex: 1; }
  .modal-btn.primary { border: 1px solid var(--cyan); background: rgba(0,245,212,0.1);
    color: var(--cyan); }
  .modal-btn.primary:hover:not(:disabled) { background: var(--cyan); color: var(--void); box-shadow: var(--glow-c); }
  .modal-btn.secondary { border: 1px solid var(--border); background: transparent; color: var(--dim); }
  .modal-btn.secondary:hover { color: var(--text); border-color: var(--dim); }
  .modal-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .modal-btn.danger { border: 1px solid var(--danger); background: rgba(255,0,110,0.08); color: var(--danger); }
  .modal-btn.danger:hover:not(:disabled) { background: var(--danger); color: var(--void); }
  .modal-info-row { font-size: 10px; color: var(--dim); display: flex; justify-content: space-between;
    padding: 4px 0; border-bottom: 1px solid rgba(26,26,58,0.3); }
  .modal-info-row:last-child { border-bottom: none; }
  .modal-info-val { color: var(--text); }

  .landing { height: 100vh; display: flex; flex-direction: column; align-items: center;
    justify-content: center; gap: 32px; position: relative; z-index: 1; }
  .landing-logo { animation: float 4s ease-in-out infinite; }
  @keyframes float { 0%,100%{transform:translateY(0)} 50%{transform:translateY(-12px)} }
  .landing-title { font-family: 'Orbitron', sans-serif; font-size: 42px; font-weight: 900;
    letter-spacing: 6px; text-align: center;
    background: linear-gradient(135deg,var(--purple) 0%,var(--cyan) 100%);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
  .landing-sub { font-size: 12px; letter-spacing: 3px; color: var(--dim); text-transform: uppercase; text-align: center; }

  .no-planet { max-width: 480px; margin: 80px auto; padding: 0 40px; text-align: center; }
  .no-planet-title { font-family:'Orbitron',sans-serif; font-size:16px; color:var(--purple); letter-spacing:3px; margin:24px 0 10px; }
  .no-planet-sub { color:var(--dim); font-size:11px; letter-spacing:1px; line-height:1.8; margin-bottom:32px; }
  .planet-name-input { background:var(--panel); border:1px solid var(--border); border-radius:3px;
    padding:10px 14px; color:var(--text); font-family:'Share Tech Mono',monospace; font-size:13px;
    letter-spacing:1px; outline:none; width:100%; text-align:center; margin-bottom:12px; }
  .create-btn { font-family:'Orbitron',sans-serif; font-size:12px; font-weight:700; letter-spacing:2px;
    padding:13px 24px; border:2px solid var(--cyan); border-radius:3px;
    background:linear-gradient(135deg,rgba(0,245,212,0.1),rgba(155,93,229,0.05));
    color:var(--cyan); cursor:pointer; transition:all 0.2s; width:100%; text-transform:uppercase; }
  .create-btn:hover:not(:disabled) { background:linear-gradient(135deg,var(--cyan),var(--purple));
    color:var(--void); box-shadow:var(--glow-c); }
  .create-btn:disabled { color:var(--dim); cursor:not-allowed; }

  .spinner { width:40px; height:40px; border:2px solid var(--border); border-top-color:var(--purple);
    border-radius:50%; animation:spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .error-msg { color:var(--danger); font-size:11px; letter-spacing:1px; margin-top:8px; }
  .success-msg { color:var(--success); font-size:11px; letter-spacing:1px; margin-top:8px; }

  .tag { font-size:9px; letter-spacing:1.5px; padding:2px 6px; border-radius:2px; text-transform:uppercase;
    background:rgba(155,93,229,0.1); border:1px solid rgba(155,93,229,0.3); color:var(--purple); }
  .pulse { animation: pulse 2s ease-in-out infinite; }
  @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:0.4} }
  @keyframes er-pulse { 0%,100%{box-shadow:0 0 6px rgba(160,96,240,0.4)} 50%{box-shadow:0 0 14px rgba(160,96,240,0.8)} }

  .wallet-adapter-button { font-family:'Share Tech Mono',monospace !important; font-size:11px !important;
    letter-spacing:1px !important; border-radius:2px !important; }

  .notice-box { background: rgba(155,93,229,0.05); border: 1px solid rgba(155,93,229,0.2);
    border-radius: 4px; padding: 10px 14px; font-size: 10px; color: var(--dim);
    letter-spacing: 1px; margin-bottom: 16px; }
  .notice-box.warn { background: rgba(255,0,110,0.05); border-color: rgba(255,0,110,0.2); color: var(--danger); }
`;

// ─── Logo SVG ─────────────────────────────────────────────────────────────────
const LogoSVG: React.FC<{ size?: number }> = ({ size = 32 }) => (
  <svg width={size} height={size} viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <linearGradient id="lg1" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stopColor="#9b5de5" />
        <stop offset="100%" stopColor="#00f5d4" />
      </linearGradient>
      <filter id="glow">
        <feGaussianBlur stdDeviation="2.5" result="coloredBlur"/>
        <feMerge><feMergeNode in="coloredBlur"/><feMergeNode in="SourceGraphic"/></feMerge>
      </filter>
    </defs>
    <rect x="18" y="18" width="48" height="48" rx="8" ry="8" transform="rotate(45 50 50)"
      stroke="url(#lg1)" strokeWidth="5" fill="none" filter="url(#glow)" />
    <rect x="26" y="26" width="36" height="36" rx="6" ry="6" transform="rotate(45 50 50)"
      stroke="url(#lg1)" strokeWidth="4" fill="none" filter="url(#glow)" opacity="0.85" />
    <rect x="36" y="36" width="20" height="20" rx="4" ry="4" transform="rotate(45 50 50)"
      stroke="url(#lg1)" strokeWidth="3.5" fill="none" filter="url(#glow)" opacity="0.7" />
  </svg>
);

// ─── Starfield ────────────────────────────────────────────────────────────────
const Starfield: React.FC = () => {
  const stars = Array.from({ length: 120 }, (_, i) => ({
    id: i, x: Math.random() * 100, y: Math.random() * 100,
    size: Math.random() * 1.8 + 0.3,
    dur: (Math.random() * 3 + 2).toFixed(1),
    delay: (Math.random() * 4).toFixed(1),
    minOp: (Math.random() * 0.2 + 0.05).toFixed(2),
  }));
  return (
    <div className="starfield">
      {stars.map(s => (
        <div key={s.id} className="star" style={{
          left: `${s.x}%`, top: `${s.y}%`, width: s.size, height: s.size,
          "--dur": `${s.dur}s`, "--delay": `${s.delay}s`, "--min-op": s.minOp,
        } as React.CSSProperties} />
      ))}
    </div>
  );
};

// ─── Resource row ─────────────────────────────────────────────────────────────
const ResRow: React.FC<{ color: string; label: string; value: bigint; cap: bigint; rate: bigint }> =
  ({ color, label, value, cap, rate }) => {
    const pct = cap > 0n ? Math.min(100, Number(value * 100n / cap)) : 0;
    return (
      <>
        <div className="res-row">
          <div className="res-name"><div className="res-dot" style={{ background: color }} />{label}</div>
          <div>
            <div className="res-val" style={{ color }}>{fmt(value)}</div>
            <div className="res-rate">+{fmt(rate)}/h</div>
          </div>
        </div>
        <div className="cap-bar">
          <div className="cap-fill" style={{ width: `${pct}%`, background: color }} />
        </div>
      </>
    );
  };

// ─── Live resource interpolation ─────────────────────────────────────────────
// Ticks resources up every second using on-chain rate + lastUpdateTs.
// Uses float math — bigint integer division floors sub-1/sec rates (e.g.
// 33/hr = 0.009/sec) to 0 every tick. This is display-only; the on-chain
// settle_resources() uses exact u64 math and will match once a tx fires.
function useInterpolatedResources(res: Resources | undefined, nowTs: number): Resources | undefined {
  return React.useMemo(() => {
    if (!res) return undefined;
    if (res.lastUpdateTs <= 0) return res;

    const dt = Math.max(0, nowTs - res.lastUpdateTs);
    if (dt === 0) return res;

    const eff = res.energyConsumption === 0n
      ? 1.0
      : Math.min(1.0, Number(res.energyProduction) / Number(res.energyConsumption));

    const produce = (current: bigint, ratePerHour: bigint, cap: bigint): bigint => {
      const gained = (Number(ratePerHour) * dt * eff) / 3600;
      const next   = Number(current) + gained;
      return BigInt(Math.floor(Math.min(next, Number(cap))));
    };

    return {
      ...res,
      metal:     produce(res.metal,     res.metalHour,     res.metalCap),
      crystal:   produce(res.crystal,   res.crystalHour,   res.crystalCap),
      deuterium: produce(res.deuterium, res.deuteriumHour, res.deuteriumCap),
    };
  }, [res, nowTs]);
}

// ─── Launch Fleet Modal ───────────────────────────────────────────────────────
interface LaunchModalProps {
  fleet: Fleet;
  res: Resources;
  onClose: () => void;
  onLaunch: (
    ships: Record<string, number>,
    cargo: { metal: bigint; crystal: bigint; deuterium: bigint },
    missionType: number,
    flightSecs: number,
    speedFactor: number,
  ) => Promise<void>;
  txBusy: boolean;
}

const COMBAT_SHIPS = ["lightFighter","heavyFighter","cruiser","battleship","battlecruiser","bomber","destroyer","deathstar"];
const CARGO_SHIPS  = ["smallCargo","largeCargo","recycler","espionageProbe","colonyShip","solarSatellite"];

const LaunchModal: React.FC<LaunchModalProps> = ({ fleet, res, onClose, onLaunch, txBusy }) => {
  const [shipQty, setShipQty]         = useState<Record<string,number>>({});
  const [missionType, setMissionType] = useState(2);
  const [cargoM, setCargoM]           = useState(0);
  const [cargoC, setCargoC]           = useState(0);
  const [cargoD, setCargoD]           = useState(0);
  const [flightH, setFlightH]         = useState(1);
  const [speed, setSpeed]             = useState(100);
  const [targetWallet, setTargetWallet] = useState("");
  const [launching, setLaunching]     = useState(false);
  const [localErr, setLocalErr]       = useState<string | null>(null);

  const getQty = (key: string) => shipQty[key] ?? 0;
  const setQty = (key: string, v: number) => setShipQty(p => ({ ...p, [key]: Math.max(0, Math.min((fleet as any)[key] ?? 0, v)) }));

  const totalSent = Object.values(shipQty).reduce((a, b) => a + b, 0);
  const cargoCap  = getQty("smallCargo") * 5000 + getQty("largeCargo") * 25000
    + getQty("recycler") * 20000 + getQty("cruiser") * 800 + getQty("battleship") * 1500;
  const cargoUsed = cargoM + cargoC + cargoD;

  const needsTarget = missionType === 2 || missionType === 5;

  const handleLaunch = async () => {
    setLocalErr(null);
    if (totalSent === 0) { setLocalErr("Select at least one ship."); return; }
    if (cargoUsed > cargoCap) { setLocalErr("Cargo exceeds capacity."); return; }
    setLaunching(true);
    try {
      await onLaunch(
        shipQty,
        { metal: BigInt(cargoM), crystal: BigInt(cargoC), deuterium: BigInt(cargoD) },
        missionType,
        flightH * 3600,
        speed,
      );
      onClose();
    } catch (e: any) {
      setLocalErr(e?.message ?? "Launch failed");
    } finally {
      setLaunching(false);
    }
  };

  const allShipKeys = [...COMBAT_SHIPS, ...CARGO_SHIPS];

  return (
    <div className="modal-backdrop" onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="modal">
        <div className="modal-title">⊹ LAUNCH FLEET</div>

        {/* Mission type */}
        <div className="modal-section">
          <div className="modal-label">Mission Type</div>
          <select className="modal-select" value={missionType}
            onChange={e => setMissionType(Number(e.target.value))}>
                        <option value={2}>TRANSPORT</option>
          </select>
        </div>

        {/* Target wallet */}
        {needsTarget && (
          <div className="modal-section">
            <div className="modal-label">Target Wallet Address</div>
            <input
              style={{ width: "100%", padding: "6px 10px", fontSize: 11, borderRadius: 2,
                background: "rgba(0,0,0,0.4)", border: "1px solid var(--border)",
                color: "var(--text)", fontFamily: "'Share Tech Mono', monospace" }}
              placeholder="Defender's wallet pubkey (Base58)"
              value={targetWallet}
              onChange={e => setTargetWallet(e.target.value.trim())}
            />
            <div style={{ fontSize: 9, color: "var(--dim)", marginTop: 4, letterSpacing: 1 }}>
              The target player's wallet — used to look up their fleet/resource PDAs on-chain.
            </div>
          </div>
        )}

        {/* Ship selection */}
        <div className="modal-section">
          <div className="modal-label">Ships  <span style={{ color: "var(--cyan)" }}>{totalSent > 0 ? `${totalSent} selected` : "none selected"}</span></div>
          <div className="modal-ship-grid">
            {allShipKeys.map(key => {
              const ship = SHIPS.find(s => s.key === key)!;
              const avail = (fleet as any)[key] as number ?? 0;
              if (avail === 0) return null;
              return (
                <div key={key} className="modal-ship-row">
                  <div>
                    <div className="modal-ship-label">{ship.icon} {ship.name}</div>
                    <div className="modal-ship-avail">Avail: {avail.toLocaleString()}</div>
                  </div>
                  <input className="modal-input" type="number" min={0} max={avail}
                    value={getQty(key) || ""}
                    placeholder="0"
                    onChange={e => setQty(key, parseInt(e.target.value) || 0)}
                  />
                </div>
              );
            })}
            {allShipKeys.every(k => ((fleet as any)[k] ?? 0) === 0) && (
              <div style={{ gridColumn: "1/-1", color: "var(--dim)", fontSize: 10, letterSpacing: 1 }}>
                No ships available. Build ships in the Shipyard first.
              </div>
            )}
          </div>
        </div>

        {/* Cargo */}
        {cargoCap > 0 && missionType !== 4 && (
          <div className="modal-section">
            <div className="modal-label">Cargo  <span style={{ color: cargoUsed > cargoCap ? "var(--danger)" : "var(--dim)" }}>
              {cargoUsed.toLocaleString()} / {cargoCap.toLocaleString()}
            </span></div>
            {[
              { label: "Metal",     color: "var(--metal)",   val: cargoM, max: Number(res.metal),     set: setCargoM },
              { label: "Crystal",   color: "var(--crystal)", val: cargoC, max: Number(res.crystal),   set: setCargoC },
              { label: "Deuterium", color: "var(--deut)",    val: cargoD, max: Number(res.deuterium), set: setCargoD },
            ].map(r => (
              <div key={r.label} className="modal-row">
                <span style={{ color: r.color, fontSize: 11 }}>{r.label} (avail: {fmt(r.max)})</span>
                <input className="modal-input" type="number" min={0} max={r.max}
                  value={r.val || ""}
                  placeholder="0"
                  onChange={e => r.set(Math.max(0, Math.min(r.max, parseInt(e.target.value) || 0)))}
                />
              </div>
            ))}
          </div>
        )}

        {/* Flight time & speed */}
        <div className="modal-section">
          <div className="modal-label">Flight Parameters</div>
          <div className="modal-row">
            <span style={{ fontSize: 11, color: "var(--dim)" }}>Flight duration (hours)</span>
            <input className="modal-input" type="number" min={1} max={240}
              value={flightH}
              onChange={e => setFlightH(Math.max(1, parseInt(e.target.value) || 1))}
            />
          </div>
          <div className="modal-row">
            <span style={{ fontSize: 11, color: "var(--dim)" }}>Speed factor (10–100%)</span>
            <input className="modal-input" type="number" min={10} max={100} step={10}
              value={speed}
              onChange={e => setSpeed(Math.max(10, Math.min(100, parseInt(e.target.value) || 100)))}
            />
          </div>
          <div style={{ fontSize: 9, color: "var(--dim)", letterSpacing: 1, marginTop: 4 }}>
            Higher speed = more deuterium fuel consumed.
          </div>
        </div>

        {/* Summary */}
        <div style={{ background: "rgba(0,0,0,0.3)", border: "1px solid var(--border)",
          borderRadius: 3, padding: "10px 12px", marginBottom: 8 }}>
          <div className="modal-info-row">
            <span>Mission</span>
            <span className="modal-info-val">{MISSION_LABELS[missionType]}</span>
          </div>
          <div className="modal-info-row">
            <span>Ships dispatched</span>
            <span className="modal-info-val">{totalSent.toLocaleString()}</span>
          </div>
          <div className="modal-info-row">
            <span>Arrive ETA</span>
            <span className="modal-info-val">{flightH}h from now</span>
          </div>
          <div className="modal-info-row">
            <span>Mission slots free</span>
            <span className="modal-info-val">{4 - fleet.activeMissions} / 4</span>
          </div>
        </div>

        {localErr && <div className="error-msg" style={{ marginBottom: 8 }}>{localErr}</div>}

        <div className="modal-footer">
          <button className="modal-btn secondary" onClick={onClose} disabled={launching || txBusy}>CANCEL</button>
          <button className="modal-btn primary" onClick={handleLaunch}
            disabled={launching || txBusy || totalSent === 0 || fleet.activeMissions >= 4}>
            {launching ? "LAUNCHING..." : "⊹ LAUNCH"}
          </button>
        </div>
      </div>
    </div>
  );
};

// ─── Attack Apply Modal ───────────────────────────────────────────────────────
// Shows when an attack mission has arrived and needs combat resolution
interface AttackApplyModalProps {
  mission: Mission;
  slotIdx: number;
  myEntityPda: string;
  onClose: () => void;
  onApply: (defenderWallet: string, slot: number) => Promise<void>;
  txBusy: boolean;
}

const AttackApplyModal: React.FC<AttackApplyModalProps> = ({ mission, slotIdx, onClose, onApply, txBusy }) => {
  const [defenderWallet, setDefenderWallet] = useState(mission.destination === "11111111111111111111111111111111" ? "" : mission.destination);
  const [applying, setApplying] = useState(false);
  const [localErr, setLocalErr] = useState<string | null>(null);

  const handleApply = async () => {
    setLocalErr(null);
    if (!defenderWallet.trim()) { setLocalErr("Enter the defender's wallet address."); return; }
    setApplying(true);
    try {
      await onApply(defenderWallet.trim(), slotIdx);
      onClose();
    } catch (e: any) {
      setLocalErr(e?.message ?? "Attack failed");
    } finally {
      setApplying(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="modal" style={{ width: 420 }}>
        <div className="modal-title" style={{ color: "var(--danger)" }}>⚔ RESOLVE BATTLE — SLOT {slotIdx}</div>
        <div className="notice-box warn">
          Fleet has arrived! Resolve combat to apply battle results on-chain.
        </div>
        <div className="modal-section">
          <div className="modal-label">Defender Wallet Address</div>
          <input
            style={{ width: "100%", padding: "6px 10px", fontSize: 11, borderRadius: 2,
              background: "rgba(0,0,0,0.4)", border: "1px solid var(--border)",
              color: "var(--text)", fontFamily: "'Share Tech Mono', monospace" }}
            placeholder="Defender's wallet pubkey"
            value={defenderWallet}
            onChange={e => setDefenderWallet(e.target.value.trim())}
          />
        </div>
        {localErr && <div className="error-msg">{localErr}</div>}
        <div className="modal-footer">
          <button className="modal-btn secondary" onClick={onClose} disabled={applying || txBusy}>CANCEL</button>
          <button className="modal-btn danger" onClick={handleApply} disabled={applying || txBusy}>
            {applying ? "RESOLVING..." : "⚔ RESOLVE BATTLE"}
          </button>
        </div>
      </div>
    </div>
  );
};

// ─── Main App ─────────────────────────────────────────────────────────────────
const App: React.FC = () => {
  const { connection }  = useConnection();
  const anchorWallet    = useAnchorWallet();
  const { connected, publicKey } = useWallet();

  const [state, setState]           = useState<PlayerState | null>(null);
  const [tab, setTab]               = useState<Tab>("overview");
  const [loading, setLoading]       = useState(false);
  const [creating, setCreating]     = useState(false);
  const [txBusy, setTxBusy]         = useState(false);
  const [sessionActive, setSessionActive] = useState(false);
  const [storedEntityPda, setStoredEntityPda] = useState<string | null>(null);
  const [planetName, setPlanetName] = useState("");
  const [error, setError]           = useState<string | null>(null);
  const [nowTs, setNowTs]           = useState(Math.floor(Date.now() / 1000));
  const [showLaunchModal, setShowLaunchModal]   = useState(false);
  const [attackModal, setAttackModal] = useState<{ mission: Mission; slotIdx: number } | null>(null);

  const clientRef = useRef<GameClient | null>(null);

  useEffect(() => {
    const id = setInterval(() => setNowTs(Math.floor(Date.now() / 1000)), 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    if (!connected || !anchorWallet || !publicKey) {
      clientRef.current = null;
      setState(null);
      return;
    }
    const provider = new AnchorProvider(connection, anchorWallet, { commitment: "confirmed" });
    clientRef.current = new GameClient(connection, provider);

    setLoading(true);
    setError(null);
    clientRef.current.findPlanet(publicKey)
      .then(s => {
        setState(s);
        if (s) {
          setStoredEntityPda(s.entityPda);
          if (s.isDelegated) {
            console.log("[APP] Planet is delegated on-chain — restoring sessionActive=true");
            setSessionActive(true);
            clientRef.current?.restoreSession();
          }
        }
      })
      .catch(e => setError(e?.message ?? "Failed to load planet"))
      .finally(() => setLoading(false));
  }, [connected, anchorWallet, publicKey, connection]);

  const refresh = useCallback(async () => {
    if (!publicKey || !clientRef.current) return;
    const isActive = clientRef.current.isSessionActive();
    try {
      const s = await clientRef.current.findPlanet(publicKey);
      if (s) {
        setState(s);
        setStoredEntityPda(s.entityPda);
        if (s.isDelegated !== sessionActive && !txBusy) {
          setSessionActive(s.isDelegated);
        }
        if (s.isDelegated && !clientRef.current?.isSessionActive()) {
          clientRef.current?.restoreSession();
        }
      } else {
        if (!isActive) setState(null);
      }
    } catch (e) { console.error("[APP] refresh() failed:", e); }
  }, [publicKey, storedEntityPda, sessionActive, txBusy]);

  useEffect(() => {
    const id = setInterval(refresh, 15_000);
    return () => clearInterval(id);
  }, [refresh]);

  const createPlanet = async () => {
    if (!clientRef.current) return;
    setError(null);
    setCreating(true);
    try {
      const s = await clientRef.current.initializePlanet(planetName.trim() || "Homeworld");
      setState(s);
      if (s) setStoredEntityPda(s.entityPda);
    } catch (e: any) {
      setError(e?.message ?? e?.toString() ?? "Failed to create planet");
    } finally {
      setCreating(false);
    }
  };

  const handleStartSession = async () => {
    if (!clientRef.current || !state || txBusy) return;
    setTxBusy(true);
    setError(null);
    try {
      await clientRef.current.startSession(new PublicKey(state.entityPda));
      setSessionActive(true);
    } catch (e: any) {
      const msg = e?.message ?? "Failed to start session";
      if (msg.includes("already delegated")) {
        setSessionActive(true);
      } else {
        setError(msg);
      }
    } finally {
      setTxBusy(false);
    }
  };

  const handleEndSession = async () => {
    if (!clientRef.current || !state || txBusy) return;
    setTxBusy(true);
    setError(null);
    try {
      await clientRef.current.endSession(new PublicKey(state.entityPda));
      setSessionActive(false);
      setTimeout(refresh, 3000);
    } catch (e: any) {
      setError(e?.message ?? "Failed to end session");
    } finally {
      setTxBusy(false);
    }
  };

  const withTx = async (label: string, fn: () => Promise<string>) => {
    if (txBusy || !clientRef.current) return;
    setTxBusy(true);
    setError(null);
    try {
      await fn();
      setTimeout(refresh, 3000);
    } catch (e: any) {
      setError(e?.message ?? `${label} failed`);
    } finally {
      setTxBusy(false);
    }
  };

  // ── Launch fleet handler ───────────────────────────────────────────────────
  const handleLaunch = async (
    ships: Record<string, number>,
    cargo: { metal: bigint; crystal: bigint; deuterium: bigint },
    missionType: number,
    flightSecs: number,
    speedFactor: number,
  ) => {
    if (!clientRef.current || !state) throw new Error("Not connected");
    setTxBusy(true);
    setError(null);
    try {
      await clientRef.current.launchFleet(
        new PublicKey(state.entityPda),
        {
          lf: ships.lightFighter, hf: ships.heavyFighter,
          cr: ships.cruiser,      bs: ships.battleship,
          bc: ships.battlecruiser, bm: ships.bomber,
          ds: ships.destroyer,    de: ships.deathstar,
          sc: ships.smallCargo,   lc: ships.largeCargo,
          rec: ships.recycler,    ep: ships.espionageProbe,
          col: ships.colonyShip,
        },
        cargo,
        missionType,
        flightSecs,
        speedFactor,
      );
      setTimeout(refresh, 3000);
    } finally {
      setTxBusy(false);
    }
  };

  // ── Apply attack handler ───────────────────────────────────────────────────
  const handleApplyAttack = async (defenderWallet: string, slot: number) => {
    if (!clientRef.current || !state) throw new Error("Not connected");
    setTxBusy(true);
    setError(null);
    try {
      let defenderPk: PublicKey;
      try { defenderPk = new PublicKey(defenderWallet); }
      catch { throw new Error("Invalid defender wallet address"); }

      const defenderInfo = await clientRef.current.findPlayerByWallet(defenderPk);
      if (!defenderInfo) throw new Error("Defender has no registered planet — check the wallet address");

      await clientRef.current.applyAttack(
        new PublicKey(state.entityPda),
        new PublicKey(defenderInfo.entityPda),
        slot,
      );
      setTimeout(refresh, 3000);
    } finally {
      setTxBusy(false);
    }
  };

  const res     = state?.resources;
  const liveRes = useInterpolatedResources(res, nowTs);
  const activeMissionCount = state?.fleet.missions.filter(m => m.missionType !== 0).length ?? 0;

  return (
    <>
      <style>{CSS}</style>
      <Starfield />

      {!connected && (
        <div className="landing">
          <div className="landing-logo"><LogoSVG size={120} /></div>
          <div>
            <div className="landing-title">CHAINED UNIVERSE</div>
            <div className="landing-sub">On-chain space strategy · Solana · BOLT ECS</div>
          </div>
          <WalletMultiButton />
        </div>
      )}

      {connected && loading && (
        <div style={{ height: "100vh", display: "flex", flexDirection: "column",
          alignItems: "center", justifyContent: "center", gap: 20 }}>
          <LogoSVG size={60} />
          <div className="spinner" />
          <div style={{ fontSize: 11, color: "var(--dim)", letterSpacing: 3 }}>LOADING UNIVERSE...</div>
        </div>
      )}

      {connected && !loading && (
        <div className="app">
          <header className="header">
            <div className="logo-area">
              <LogoSVG size={28} />
              <span className="game-title">CHAINED UNIVERSE</span>
            </div>
            <div className="header-right">
              <span className="chain-tag">DEVNET</span>
              {publicKey && (
                <span className="chain-tag">
                  {publicKey.toBase58().slice(0,4)}…{publicKey.toBase58().slice(-4)}
                </span>
              )}
              {state && (
                sessionActive ? (
                  <button onClick={handleEndSession} disabled={txBusy}
                    title="Commit state to Solana devnet and end instant session"
                    style={{ fontFamily:"'Share Tech Mono',monospace", fontSize:11, letterSpacing:1,
                      padding:"7px 14px", borderRadius:2, border:"1px solid #a060f0",
                      background:"rgba(160,96,240,0.15)", color:"#a060f0",
                      cursor:txBusy?"not-allowed":"pointer", animation:"er-pulse 2s ease-in-out infinite" }}>
                    ⚡ SAVE & EXIT
                  </button>
                ) : (
                  <button onClick={handleStartSession} disabled={txBusy}
                    title="Delegate to Ephemeral Rollup for instant transactions"
                    style={{ fontFamily:"'Share Tech Mono',monospace", fontSize:11, letterSpacing:1,
                      padding:"7px 14px", borderRadius:2, border:"1px solid #4a5568",
                      background:"transparent", color:"#a060f0", cursor:txBusy?"not-allowed":"pointer" }}>
                    ⚡ START SESSION
                  </button>
                )
              )}
              {sessionActive && (
                <>
                  <span style={{ fontSize:10, letterSpacing:1, padding:"4px 8px", borderRadius:2,
                    background:"rgba(160,96,240,0.1)", border:"1px solid rgba(160,96,240,0.4)",
                    color:"#a060f0", animation:"er-pulse 2s ease-in-out infinite" }}>⚡ ER ACTIVE</span>
                  <span style={{ fontSize:10, letterSpacing:1, padding:"4px 8px", borderRadius:2,
                    background:"rgba(240,160,0,0.1)", border:"1px solid rgba(240,160,0,0.3)",
                    color:"#f0a000" }} title="End session first to save state to Solana.">
                    ⚠ DON'T REFRESH
                  </span>
                </>
              )}
              <WalletMultiButton />
            </div>
          </header>

          <aside className="sidebar">
            {state ? (
              <>
                <div className="planet-card">
                  <div className="planet-coords">
                    [{state.planet.galaxy}:{state.planet.system}:{state.planet.position}]
                  </div>
                  <div className="planet-name">{state.planet.name || "Unknown"}</div>
                  <div className="planet-meta">
                    {state.planet.diameter.toLocaleString()} km · {state.planet.temperature}°C
                  </div>
                  <div className="fields-bar">
                    <div className="fields-fill"
                      style={{ width: `${(state.planet.usedFields / state.planet.maxFields) * 100}%` }} />
                  </div>
                  <div className="fields-label">
                    <span>Fields</span>
                    <span>{state.planet.usedFields}/{state.planet.maxFields}</span>
                  </div>
                </div>

                {liveRes && (
                  <div className="res-panel">
                    <div className="res-label">Resources</div>
                    <ResRow color="var(--metal)"  label="Metal"     value={liveRes.metal}     cap={liveRes.metalCap}     rate={liveRes.metalHour} />
                    <ResRow color="var(--crystal)" label="Crystal"   value={liveRes.crystal}   cap={liveRes.crystalCap}   rate={liveRes.crystalHour} />
                    <ResRow color="var(--deut)"   label="Deuterium" value={liveRes.deuterium} cap={liveRes.deuteriumCap} rate={liveRes.deuteriumHour} />
                    <div className="energy-row">
                      <span style={{ color:"var(--dim)", fontSize:10, letterSpacing:1 }}>⚡ ENERGY</span>
                      <span style={{ fontSize:11, fontWeight:600,
                        color: energyEfficiency(liveRes) >= 100 ? "var(--success)"
                             : energyEfficiency(liveRes) >= 60  ? "var(--warn)" : "var(--danger)" }}>
                        {fmt(liveRes.energyProduction)}/{fmt(liveRes.energyConsumption)} ({energyEfficiency(liveRes)}%)
                      </span>
                    </div>
                  </div>
                )}

                <nav className="nav">
                  {([
                    { id: "overview",  icon: "◈", label: "Overview",  badge: 0 },
                    { id: "buildings", icon: "⬡", label: "Buildings", badge: 0 },
                    { id: "shipyard",  icon: "🚀", label: "Shipyard",  badge: 0 },
                    { id: "fleet",     icon: "◉", label: "Fleet",     badge: 0 },
                    { id: "missions",  icon: "⊹", label: "Missions",  badge: activeMissionCount },
                  ] as Array<{ id: Tab; icon: string; label: string; badge: number }>).map(item => (
                    <div key={item.id}
                      className={`nav-item${tab === item.id ? " active" : ""}`}
                      onClick={() => setTab(item.id)}>
                      <span>{item.icon}</span>
                      {item.label}
                      {item.badge > 0 && <span className="nav-badge">{item.badge}</span>}
                    </div>
                  ))}
                </nav>
              </>
            ) : (
              <div style={{ padding: 20, color: "var(--dim)", fontSize: 11, letterSpacing: 1 }}>
                <div className="pulse">No planet found.</div>
                <div style={{ marginTop: 8, fontSize: 10, color: "var(--purple)" }}>
                  {publicKey?.toBase58().slice(0, 12)}…
                </div>
              </div>
            )}
          </aside>

          <main className="main">
            {error && (
              <div style={{ color:"var(--danger)", fontSize:11, letterSpacing:1, marginBottom:16 }}>
                {error}
              </div>
            )}

            {!state ? (
              <NoPlanetView
                planetName={planetName}
                onNameChange={setPlanetName}
                onCreate={createPlanet}
                creating={creating}
                error={error}
              />
            ) : tab === "overview" ? (
              <OverviewTab state={state} res={liveRes} nowTs={nowTs}
                onFinishBuild={() => withTx("Finish build", () =>
                  clientRef.current!.finishBuild(new PublicKey(state.entityPda))
                )}
                txBusy={txBusy}
              />
            ) : tab === "buildings" ? (
              <BuildingsTab state={state} res={liveRes} nowTs={nowTs}
                onStartBuild={(idx) => withTx("Start build", () =>
                  clientRef.current!.startBuild(new PublicKey(state.entityPda), idx)
                )}
                onFinishBuild={() => withTx("Finish build", () =>
                  clientRef.current!.finishBuild(new PublicKey(state.entityPda))
                )}
                txBusy={txBusy}
              />
            ) : tab === "shipyard" ? (
              <ShipyardTab
                state={state}
                res={liveRes}
                txBusy={txBusy}
                onBuildShip={(shipType, qty) => withTx("Build ship", () =>
                  clientRef.current!.buildShip(new PublicKey(state.entityPda), shipType, qty)
                )}
              />
            ) : tab === "fleet" ? (
              <FleetTab
                fleet={state.fleet}
                res={liveRes}
                txBusy={txBusy}
                onOpenLaunch={() => setShowLaunchModal(true)}
              />
            ) : (
              <MissionsTab
                fleet={state.fleet}
                nowTs={nowTs}
                txBusy={txBusy}
                onOpenAttack={(mission, slot) => setAttackModal({ mission, slotIdx: slot })}
              />
            )}
          </main>
        </div>
      )}

      {/* Launch Fleet Modal */}
      {showLaunchModal && state && liveRes && (
        <LaunchModal
          fleet={state.fleet}
          res={liveRes}
          onClose={() => setShowLaunchModal(false)}
          onLaunch={handleLaunch}
          txBusy={txBusy}
        />
      )}

      {/* Attack Apply Modal */}
      {attackModal && state && (
        <AttackApplyModal
          mission={attackModal.mission}
          slotIdx={attackModal.slotIdx}
          myEntityPda={state.entityPda}
          onClose={() => setAttackModal(null)}
          onApply={handleApplyAttack}
          txBusy={txBusy}
        />
      )}
    </>
  );
};

// ─── No Planet View ───────────────────────────────────────────────────────────
const NoPlanetView: React.FC<{
  planetName: string; onNameChange: (v: string) => void;
  onCreate: () => void; creating: boolean; error: string | null;
}> = ({ planetName, onNameChange, onCreate, creating, error }) => (
  <div className="no-planet">
    <LogoSVG size={64} />
    <div className="no-planet-title">NO PLANET FOUND</div>
    <div className="no-planet-sub">
      This wallet has no initialized planet on-chain.<br />Create your homeworld to begin.
    </div>
    <input className="planet-name-input" type="text" placeholder="Planet name (optional)"
      value={planetName} onChange={e => onNameChange(e.target.value)} maxLength={19} />
    <button className="create-btn" onClick={onCreate} disabled={creating}>
      {creating ? "TRANSMITTING TO CHAIN..." : "⊹ INITIALIZE HOMEWORLD"}
    </button>
    {error && <div className="error-msg">{error}</div>}
    <div style={{ fontSize:10, color:"var(--dim)", letterSpacing:1, marginTop:12 }}>
      Requires 3 wallet approvals · Rent paid in SOL
    </div>
  </div>
);

// ─── Overview Tab ─────────────────────────────────────────────────────────────
const OverviewTab: React.FC<{
  state: PlayerState; res?: Resources; nowTs: number;
  onFinishBuild: () => void; txBusy: boolean;
}> = ({ state, res, nowTs, onFinishBuild, txBusy }) => {
  const { planet, fleet } = state;
  const buildInProgress = planet.buildFinishTs > 0 && planet.buildQueueItem !== 255;
  const buildSecsLeft   = Math.max(0, planet.buildFinishTs - nowTs);
  const buildBuilding   = BUILDINGS.find(b => b.idx === planet.buildQueueItem);
  const totalFleet      = SHIPS.reduce((s, sh) => s + ((fleet as any)[sh.key] ?? 0), 0);

  return (
    <div>
      <div className="section-title">COMMAND OVERVIEW</div>
      {buildInProgress && buildBuilding && (
        <div className="build-queue-banner">
          <div>
            <div className="build-queue-label">⚙ Building</div>
            <div className="build-queue-item-name">
              {buildBuilding.icon} {buildBuilding.name} → Lv {planet.buildQueueTarget}
            </div>
          </div>
          <div className="build-queue-right">
            {buildSecsLeft === 0 ? (
              <button onClick={onFinishBuild} disabled={txBusy}
                style={{ fontFamily:"'Orbitron',sans-serif", fontSize:11, padding:"8px 16px",
                  border:"1px solid var(--success)", background:"rgba(6,214,160,0.1)",
                  color:"var(--success)", cursor:"pointer", borderRadius:2, letterSpacing:1 }}>
                COLLECT
              </button>
            ) : (
              <>
                <div className="build-queue-eta">{fmtCountdown(buildSecsLeft)}</div>
                <div style={{ fontSize:9, color:"var(--dim)", marginTop:2, letterSpacing:1 }}>REMAINING</div>
              </>
            )}
          </div>
        </div>
      )}

      <div className="grid-4" style={{ marginBottom: 28 }}>
        <div className="card">
          <div className="card-label">Coordinates</div>
          <div className="card-value" style={{ fontSize: 14 }}>
            [{planet.galaxy}:{planet.system}:{planet.position}]
          </div>
        </div>
        <div className="card">
          <div className="card-label">Fleet Size</div>
          <div className="card-value">{totalFleet.toLocaleString()}</div>
          <div className="card-sub">{fleet.activeMissions} missions active</div>
        </div>
        <div className="card">
          <div className="card-label">Metal / hr</div>
          <div className="card-value" style={{ color:"var(--metal)" }}>{res ? fmt(res.metalHour) : "—"}</div>
        </div>
        <div className="card">
          <div className="card-label">Crystal / hr</div>
          <div className="card-value" style={{ color:"var(--crystal)" }}>{res ? fmt(res.crystalHour) : "—"}</div>
        </div>
      </div>

      <div className="grid-2">
        <div>
          <div className="section-title">PLANET INFO</div>
          <div className="card">
            {[
              ["Name",        planet.name || "Unknown"],
              ["Diameter",    `${planet.diameter.toLocaleString()} km`],
              ["Temperature", `${planet.temperature}°C`],
              ["Fields",      `${planet.usedFields} / ${planet.maxFields}`],
              ["Galaxy",      planet.galaxy],
              ["System",      planet.system],
              ["Position",    planet.position],
            ].map(([k, v]) => (
              <div key={String(k)} className="stat-row">
                <span className="stat-key">{k}</span>
                <span className="stat-val">{v}</span>
              </div>
            ))}
          </div>
        </div>
        <div>
          <div className="section-title">KEY BUILDINGS</div>
          <div className="card">
            {BUILDINGS.slice(0, 8).map(b => (
              <div key={b.idx} className="stat-row">
                <span className="stat-key">{b.icon} {b.name}</span>
                <span className="stat-val" style={{ color:"var(--purple)", fontFamily:"'Orbitron',sans-serif" }}>
                  Lv {(planet as any)[b.key] ?? 0}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

// ─── Buildings Tab ────────────────────────────────────────────────────────────
const BuildingsTab: React.FC<{
  state: PlayerState; res?: Resources; nowTs: number;
  onStartBuild: (idx: number) => void; onFinishBuild: () => void; txBusy: boolean;
}> = ({ state, res, nowTs, onStartBuild, onFinishBuild, txBusy }) => {
  const { planet } = state;
  const buildInProgress = planet.buildFinishTs > 0 && planet.buildQueueItem !== 255;
  const buildSecsLeft   = Math.max(0, planet.buildFinishTs - nowTs);

  return (
    <div>
      <div className="section-title">INFRASTRUCTURE</div>
      {buildInProgress && (
        <div className="build-queue-banner" style={{ marginBottom: 20 }}>
          <div>
            <div className="build-queue-label">⚙ Constructing</div>
            <div className="build-queue-item-name">
              {BUILDINGS.find(b => b.idx === planet.buildQueueItem)?.icon}{" "}
              {BUILDINGS.find(b => b.idx === planet.buildQueueItem)?.name} → Lv {planet.buildQueueTarget}
            </div>
          </div>
          <div className="build-queue-right">
            <div className="build-queue-eta">{fmtCountdown(buildSecsLeft)}</div>
            {buildSecsLeft === 0 && (
              <div style={{ fontSize:9, color:"var(--success)", marginTop:2, letterSpacing:1 }}>READY TO FINISH</div>
            )}
          </div>
        </div>
      )}

      <div className="grid-3">
        {BUILDINGS.filter(b => b.idx !== 11 && b.idx !== 12).map(b => {
          const level     = (planet as any)[b.key] as number ?? 0;
          const nextLevel = level + 1;
          const [cm, cc, cd] = upgradeCost(b.idx, level);
          const secs      = buildTimeSecs(b.idx, nextLevel, planet.roboticsFactory);
          const hasMetal   = res ? res.metal    >= BigInt(cm) : false;
          const hasCrystal = res ? res.crystal  >= BigInt(cc) : false;
          const hasDeut    = res ? res.deuterium >= BigInt(cd) : false;
          const canAfford  = hasMetal && hasCrystal && hasDeut;
          const isQueued   = buildInProgress && planet.buildQueueItem === b.idx;
          const isReady    = isQueued && buildSecsLeft === 0;

          let btnClass = "build-btn no-funds";
          let btnText  = "INSUFFICIENT";
          if (isReady)                              { btnClass = "build-btn finish-btn"; btnText = "FINISH BUILD"; }
          else if (isQueued)                        { btnClass = "build-btn building-now"; btnText = fmtCountdown(buildSecsLeft); }
          else if (!buildInProgress && canAfford)   { btnClass = "build-btn can-build"; btnText = `BUILD  ${fmtCountdown(secs)}`; }

          return (
            <div key={b.idx} className="building-card">
              <div className="building-header">
                <div className="building-icon-name">
                  <span className="building-icon">{b.icon}</span>
                  <span className="building-name">{b.name}</span>
                </div>
                <span className="building-level">{level}</span>
              </div>
              <div className="building-costs">
                {cm > 0 && <div className="building-cost-row"><span>Metal</span><span className={hasMetal ? "cost-ok":"cost-bad"}>{fmt(cm)}</span></div>}
                {cc > 0 && <div className="building-cost-row"><span>Crystal</span><span className={hasCrystal?"cost-ok":"cost-bad"}>{fmt(cc)}</span></div>}
                {cd > 0 && <div className="building-cost-row"><span>Deuterium</span><span className={hasDeut?"cost-ok":"cost-bad"}>{fmt(cd)}</span></div>}
              </div>
              <button className={btnClass} disabled={(isQueued && !isReady) || txBusy}
                onClick={() => isReady ? onFinishBuild() : onStartBuild(b.idx)}>
                {btnText}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
};

// ─── Shipyard Tab ─────────────────────────────────────────────────────────────
const ShipyardTab: React.FC<{
  state: PlayerState; res?: Resources; txBusy: boolean;
  onBuildShip: (shipType: number, qty: number) => void;
}> = ({ state, res, txBusy, onBuildShip }) => {
  const [quantities, setQuantities] = useState<Record<string, number>>({});
  const { planet } = state;
  const shipyardLevel = planet.shipyard;

  const getQty = (key: string) => quantities[key] ?? 1;
  const setQty = (key: string, v: number) => setQuantities(p => ({ ...p, [key]: Math.max(1, v) }));

  const canAfford = (cost: { m: number; c: number; d: number }, qty: number): [boolean, boolean, boolean] => {
    if (!res) return [false, false, false];
    const nm = res.metal    >= BigInt(cost.m * qty);
    const nc = res.crystal  >= BigInt(cost.c * qty);
    const nd = res.deuterium >= BigInt(cost.d * qty);
    return [nm, nc, nd];
  };

  return (
    <div>
      <div className="section-title">SHIPYARD</div>

      {shipyardLevel === 0 ? (
        <div className="notice-box" style={{ textAlign: "center", padding: "32px 20px" }}>
          <div style={{ fontSize: 28, marginBottom: 10 }}>🚀</div>
          <div style={{ fontSize: 12, color: "var(--purple)", letterSpacing: 2, marginBottom: 8 }}>SHIPYARD NOT BUILT</div>
          <div style={{ fontSize: 10, color: "var(--dim)" }}>Build a Shipyard in the Buildings tab to unlock ship construction.</div>
        </div>
      ) : (
        <>
          <div style={{ fontSize: 10, color: "var(--dim)", letterSpacing: 1, marginBottom: 20 }}>
            Shipyard Lv {shipyardLevel} · All ships built and added to hangar instantly.
          </div>

          <div style={{ marginBottom: 28 }}>
            <div className="section-title" style={{ fontSize: 10 }}>COMBAT SHIPS</div>
            <div className="grid-3">
              {SHIPS.filter(s => s.atk > 0 && s.key !== "solarSatellite").map(ship => {
                const typeIdx  = SHIP_TYPE_IDX[ship.key] ?? -1;
                const qty      = getQty(ship.key);
                const [nm, nc, nd] = canAfford(ship.cost, qty);
                const allOk    = nm && nc && nd;
                const current  = (state.fleet as any)[ship.key] as number ?? 0;
                return (
                  <div key={ship.key} className="ship-build-card">
                    <div className="ship-build-header">
                      <div className="ship-build-icon-name">
                        <span className="ship-build-icon">{ship.icon}</span>
                        <div>
                          <div className="ship-build-name">{ship.name}</div>
                          <div className="ship-build-stats">
                            {ship.atk > 0 && <span>⚔ {fmt(ship.atk)}</span>}
                            {ship.cargo > 0 && <span>📦 {fmt(ship.cargo)}</span>}
                          </div>
                        </div>
                      </div>
                      <div className={`ship-build-count${current === 0 ? " zero" : ""}`}>{current.toLocaleString()}</div>
                    </div>
                    <div style={{ fontSize: 10, color: "var(--dim)", display: "flex", flexDirection: "column", gap: 2 }}>
                      {ship.cost.m > 0 && <div style={{ display:"flex", justifyContent:"space-between" }}>
                        <span>Metal</span><span style={{ color: nm ? "var(--text)" : "var(--danger)" }}>{fmt(ship.cost.m * qty)}</span>
                      </div>}
                      {ship.cost.c > 0 && <div style={{ display:"flex", justifyContent:"space-between" }}>
                        <span>Crystal</span><span style={{ color: nc ? "var(--text)" : "var(--danger)" }}>{fmt(ship.cost.c * qty)}</span>
                      </div>}
                      {ship.cost.d > 0 && <div style={{ display:"flex", justifyContent:"space-between" }}>
                        <span>Deuterium</span><span style={{ color: nd ? "var(--text)" : "var(--danger)" }}>{fmt(ship.cost.d * qty)}</span>
                      </div>}
                    </div>
                    <div className="ship-qty-row">
                      <input className="qty-input" type="number" min={1} value={qty}
                        onChange={e => setQty(ship.key, parseInt(e.target.value) || 1)} />
                      <button className="ship-build-btn"
                        disabled={!allOk || txBusy || typeIdx < 0}
                        onClick={() => onBuildShip(typeIdx, qty)}>
                        BUILD ×{qty}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          <div>
            <div className="section-title" style={{ fontSize: 10 }}>UTILITY SHIPS</div>
            <div className="grid-3">
              {SHIPS.filter(s => s.key === "smallCargo" || s.key === "largeCargo" || s.key === "recycler" || s.key === "colonyShip").map(ship => {
                const typeIdx  = SHIP_TYPE_IDX[ship.key] ?? -1;
                const qty      = getQty(ship.key);
                const [nm, nc, nd] = canAfford(ship.cost, qty);
                const allOk    = nm && nc && nd;
                const current  = (state.fleet as any)[ship.key] as number ?? 0;
                return (
                  <div key={ship.key} className="ship-build-card">
                    <div className="ship-build-header">
                      <div className="ship-build-icon-name">
                        <span className="ship-build-icon">{ship.icon}</span>
                        <div>
                          <div className="ship-build-name">{ship.name}</div>
                          <div className="ship-build-stats">
                            {ship.cargo > 0 && <span>📦 {fmt(ship.cargo)}</span>}
                          </div>
                        </div>
                      </div>
                      <div className={`ship-build-count${current === 0 ? " zero" : ""}`}>{current.toLocaleString()}</div>
                    </div>
                    <div style={{ fontSize: 10, color: "var(--dim)", display: "flex", flexDirection: "column", gap: 2 }}>
                      {ship.cost.m > 0 && <div style={{ display:"flex", justifyContent:"space-between" }}>
                        <span>Metal</span><span style={{ color: nm ? "var(--text)" : "var(--danger)" }}>{fmt(ship.cost.m * qty)}</span>
                      </div>}
                      {ship.cost.c > 0 && <div style={{ display:"flex", justifyContent:"space-between" }}>
                        <span>Crystal</span><span style={{ color: nc ? "var(--text)" : "var(--danger)" }}>{fmt(ship.cost.c * qty)}</span>
                      </div>}
                      {ship.cost.d > 0 && <div style={{ display:"flex", justifyContent:"space-between" }}>
                        <span>Deuterium</span><span style={{ color: nd ? "var(--text)" : "var(--danger)" }}>{fmt(ship.cost.d * qty)}</span>
                      </div>}
                    </div>
                    <div className="ship-qty-row">
                      <input className="qty-input" type="number" min={1} value={qty}
                        onChange={e => setQty(ship.key, parseInt(e.target.value) || 1)} />
                      <button className="ship-build-btn"
                        disabled={!allOk || txBusy || typeIdx < 0}
                        onClick={() => onBuildShip(typeIdx, qty)}>
                        BUILD ×{qty}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </>
      )}
    </div>
  );
};

// ─── Fleet Tab ────────────────────────────────────────────────────────────────
const FleetTab: React.FC<{
  fleet: Fleet; res?: Resources; txBusy: boolean; onOpenLaunch: () => void;
}> = ({ fleet, res, txBusy, onOpenLaunch }) => {
  const totalShips = SHIPS.reduce((s, sh) => s + ((fleet as any)[sh.key] ?? 0), 0);
  const slotsAvail = 4 - fleet.activeMissions;
  return (
    <div>
      <div className="section-title">FLEET COMMAND</div>
      <div className="grid-4" style={{ marginBottom: 24 }}>
        <div className="card"><div className="card-label">Total Ships</div><div className="card-value">{totalShips.toLocaleString()}</div></div>
        <div className="card"><div className="card-label">Active Missions</div><div className="card-value">{fleet.activeMissions}</div></div>
        <div className="card">
          <div className="card-label">Mission Slots</div>
          <div className="card-value">{slotsAvail} / 4</div>
          <div className="card-sub">Available</div>
        </div>
        <div className="card">
          <div className="card-label">Cargo Capacity</div>
          <div className="card-value" style={{ fontSize: 14 }}>
            {fmt(fleet.smallCargo * 5000 + fleet.largeCargo * 25000 + fleet.recycler * 20000 + fleet.cruiser * 800 + fleet.battleship * 1500)}
          </div>
        </div>
      </div>

      <div style={{ display: "flex", gap: 12, marginBottom: 24 }}>
        <button
          onClick={onOpenLaunch}
          disabled={txBusy || slotsAvail === 0 || totalShips === 0}
          style={{ fontFamily:"'Orbitron',sans-serif", fontSize:11, fontWeight:700,
            letterSpacing:2, padding:"10px 24px", borderRadius:3,
            border:"2px solid var(--cyan)", background:"linear-gradient(135deg,rgba(0,245,212,0.1),rgba(155,93,229,0.05))",
            color:"var(--cyan)", cursor:(txBusy || slotsAvail === 0 || totalShips === 0) ? "not-allowed" : "pointer",
            transition:"all 0.2s", opacity:(slotsAvail === 0 || totalShips === 0) ? 0.5 : 1 }}>
          ⊹ LAUNCH FLEET
        </button>
        {slotsAvail === 0 && <span style={{ fontSize:10, color:"var(--dim)", alignSelf:"center", letterSpacing:1 }}>All mission slots occupied</span>}
        {totalShips === 0 && <span style={{ fontSize:10, color:"var(--dim)", alignSelf:"center", letterSpacing:1 }}>No ships in hangar — build ships in Shipyard</span>}
      </div>

      <div className="section-title">HANGAR</div>
      <div className="grid-4">
        {SHIPS.map(s => {
          const count = (fleet as any)[s.key] ?? 0;
          return (
            <div key={s.key} className="ship-card">
              <div className="ship-icon">{s.icon}</div>
              <div className="ship-name">{s.name.toUpperCase()}</div>
              <div className={`ship-count${count === 0 ? " zero" : ""}`}>{count.toLocaleString()}</div>
              {s.cargo > 0 && count > 0 && <div style={{ fontSize:9, color:"var(--dim)" }}>{fmt(count * s.cargo)} cargo</div>}
              {s.atk > 0 && count > 0 && <div style={{ fontSize:9, color:"var(--danger)" }}>⚔ {fmt(s.atk * count)}</div>}
            </div>
          );
        })}
      </div>
    </div>
  );
};

// ─── Missions Tab ─────────────────────────────────────────────────────────────
const MissionsTab: React.FC<{
  fleet: Fleet; nowTs: number; txBusy: boolean;
  onOpenAttack: (mission: Mission, slotIdx: number) => void;
}> = ({ fleet, nowTs, txBusy, onOpenAttack }) => {
  const activeMissions = fleet.missions.map((m, i) => ({ m, i })).filter(({ m }) => m.missionType !== 0);

  if (activeMissions.length === 0) {
    return (
      <div>
        <div className="section-title">ACTIVE MISSIONS</div>
        <div style={{ textAlign:"center", padding:"60px 20px", color:"var(--dim)", fontSize:12, letterSpacing:1 }}>
          <div style={{ fontSize:32, marginBottom:12 }}>⊹</div>
          <div>No missions in flight</div>
          <div style={{ fontSize:10, marginTop:8 }}>Launch a fleet from the Fleet tab to begin.</div>
        </div>
      </div>
    );
  }

  return (
    <div>
      <div className="section-title">ACTIVE MISSIONS</div>
      {activeMissions.map(({ m, i }) => {
        const progress  = missionProgress(m, nowTs);
        const returning = m.applied;
        const etaSecs   = returning ? Math.max(0, m.returnTs - nowTs) : Math.max(0, m.arriveTs - nowTs);
        const typeLabel = MISSION_LABELS[m.missionType] ?? "UNKNOWN";
        const typeClass = m.missionType === 2 ? "transport" : "other";

        // Attack arrived but not yet resolved
        const needsResolution = false;
        // Returning mission that has arrived back
        const returnedHome    = m.applied && nowTs >= m.returnTs;

        const ships = [
          { label: "LF", n: m.sLightFighter }, { label: "HF", n: m.sHeavyFighter },
          { label: "CR", n: m.sCruiser },      { label: "BS", n: m.sBattleship },
          { label: "BC", n: m.sBattlecruiser }, { label: "BM", n: m.sBomber },
          { label: "DS", n: m.sDestroyer },    { label: "DE", n: m.sDeathstar },
          { label: "SC", n: m.sSmallCargo },   { label: "LC", n: m.sLargeCargo },
          { label: "REC", n: m.sRecycler },    { label: "EP", n: m.sEspionageProbe },
          { label: "COL", n: m.sColonyShip },
        ].filter(s => s.n > 0);

        const hasCargo = m.cargoMetal > 0n || m.cargoCrystal > 0n || m.cargoDeuterium > 0n;

        return (
          <div key={i} className="mission-card">
            <div className="mission-header">
              <div style={{ display:"flex", alignItems:"center", gap:10 }}>
                <span className={`mission-type-badge ${typeClass}`}>{typeLabel}</span>
                <span className="tag">SLOT {i}</span>
                {needsResolution && (
                  <span style={{ fontSize:9, color:"var(--danger)", letterSpacing:1,
                    padding:"2px 6px", border:"1px solid rgba(255,0,110,0.4)",
                    borderRadius:2, background:"rgba(255,0,110,0.08)" }}>
                    ⚔ RESOLVE REQUIRED
                  </span>
                )}
                {returnedHome && (
                  <span style={{ fontSize:9, color:"var(--success)", letterSpacing:1,
                    padding:"2px 6px", border:"1px solid rgba(6,214,160,0.4)",
                    borderRadius:2, background:"rgba(6,214,160,0.08)" }}>
                    ✓ RETURNED
                  </span>
                )}
              </div>
              {returning && !returnedHome && <span className="mission-returning">↩ RETURNING</span>}
            </div>

            <div className="progress-bar">
              <div className={`progress-fill ${returning ? "returning" : "outbound"}`} style={{ width: `${progress}%` }} />
            </div>
            <div className="mission-info">
              <span>{returning ? "Return ETA" : "Arrive ETA"}</span>
              <span className="mission-eta">{etaSecs <= 0 ? (needsResolution ? "ARRIVED — RESOLVE BATTLE" : "ARRIVED") : fmtCountdown(etaSecs)}</span>
            </div>
            <div className="mission-info" style={{ marginTop: 4 }}>
              <span>Progress</span><span>{progress}%</span>
            </div>

            <div className="mission-ships">
              {ships.map(s => (
                <span key={s.label} className="mission-ship-badge">{s.label} ×{s.n.toLocaleString()}</span>
              ))}
            </div>

            {hasCargo && (
              <div style={{ marginTop:10, fontSize:10, color:"var(--dim)", display:"flex", gap:16 }}>
                {m.cargoMetal > 0n     && <span style={{ color:"var(--metal)"   }}>⛏ {fmt(m.cargoMetal)}</span>}
                {m.cargoCrystal > 0n   && <span style={{ color:"var(--crystal)" }}>💎 {fmt(m.cargoCrystal)}</span>}
                {m.cargoDeuterium > 0n && <span style={{ color:"var(--deut)"    }}>🧪 {fmt(m.cargoDeuterium)}</span>}
              </div>
            )}

            {/* Attack: resolve battle on arrival */}
            {needsResolution && (
              <button className="apply-btn danger" disabled={txBusy}
                onClick={() => onOpenAttack(m, i)}>
                ⚔ RESOLVE BATTLE
              </button>
            )}

            {/* Returned: fleet is back, will be auto-cleared on next chain poll */}
            {returnedHome && (
              <div style={{ marginTop:10, fontSize:10, color:"var(--success)", letterSpacing:1 }}>
                ✓ Fleet returned home. Resources credited on-chain. Refresh to update.
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default App;