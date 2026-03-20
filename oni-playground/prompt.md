You are given a complete engineering implementation task.
Produce everything specified. No placeholders. No "you could add X later."
Code must run end-to-end unmodified.

═══════════════════════════════════════════════════════════════════════
TECH STACK — MANDATORY
═══════════════════════════════════════════════════════════════════════

Language:     Python 3.10+
Allowed libs: numpy, matplotlib, scipy (only scipy.spatial.transform 
              for quaternion validation ONLY), pygame (visualisation),
              dataclasses, typing, argparse, math, random, time
Forbidden:    any control/robotics/physics library (no pybullet, 
              no sympy, no control, no gymnasium)
Structure:
  spacecraft_sim/
  ├── main.py          # entry point, argparse, main loop
  ├── dynamics.py      # RK4, equations of motion
  ├── controller.py    # attitude control law
  ├── disturbances.py  # gravity gradient, noise
  ├── visualiser.py    # pygame 3D wireframe + telemetry
  ├── montecarlo.py    # MC runner + statistics
  └── analysis.py      # post-run plots (matplotlib)

═══════════════════════════════════════════════════════════════════════
PART 1 — DYNAMICS
═══════════════════════════════════════════════════════════════════════

1.1  Derive (in comments) and implement Euler's rotational EOM:
       I·ω̇ = -ω × (I·ω) + τ_total
     τ_total = τ_control + τ_disturbance

1.2  Derive (in comments) and implement quaternion kinematics:
       q̇ = 0.5 · Ξ(q) · ω
     Scalar-first convention: q = [q0, q1, q2, q3].
     Define Ξ(q) as the 4×3 matrix explicitly in code.
     Re-normalise q after every RK4 step.

1.3  RK4 integrator — implement from scratch in dynamics.py:
       state x = [q0,q1,q2,q3, wx,wy,wz]  (len=7)
       dt = 0.005s
       Inertia: I = diag(10.0, 15.0, 8.0) kg·m²

═══════════════════════════════════════════════════════════════════════
PART 2 — NONLINEAR CONTROL
═══════════════════════════════════════════════════════════════════════

2.1  Implement Lyapunov-based quaternion error controller:
       q_err  = q_d^{-1} ⊗ q          # quaternion multiply
       τ_ctrl = -kp · q_err[1:4] - kd · ω_meas
     Default gains: kp=5.0, kd=10.0
     Actuator saturation: if ||τ_ctrl|| > τ_max=5.0 N·m,
       scale τ_ctrl *= τ_max / ||τ_ctrl||   # preserve direction

2.2  Manual gain tuning via keyboard (implement in main loop):
       UP/DOWN    →  kp ± 0.5
       LEFT/RIGHT →  kd ± 0.5
       R          →  reset to initial conditions
       SPACE      →  toggle controller ON/OFF
       M          →  run Monte Carlo (200 trials, non-blocking thread)
       S          →  save current plots to ./output/
       Q/ESC      →  quit

2.3  Target attitude: configurable via argparse --target-quat
     Default: q_d = [1,0,0,0]

═══════════════════════════════════════════════════════════════════════
PART 3 — DISTURBANCES
═══════════════════════════════════════════════════════════════════════

3.1  Gravity-gradient torque (compute in disturbances.py):
       τ_gg = (3μ/r³) · n̂ × (I · n̂)
       μ = 3.986e14 m³/s²,  r = 6.771e6 m
       n̂ = [0,0,1] body-frame nadir vector

3.2  Gyroscope noise:
       ω_meas = ω_true + η,  η ~ N(0, (0.01)²·I₃)
     Controller receives ω_meas. Dynamics propagate ω_true.

3.3  Toggle disturbances via keyboard:
       G  →  toggle gravity gradient ON/OFF
       N  →  toggle gyro noise ON/OFF

═══════════════════════════════════════════════════════════════════════
PART 4 — REAL-TIME VISUALISER (visualiser.py, pygame)
═══════════════════════════════════════════════════════════════════════

4.1  LEFT PANEL — 3D wireframe spacecraft body:
       Represent spacecraft as a box: 2m × 1.5m × 1m
       Define 8 vertices in body frame.
       Rotate using current quaternion → apply simple 
       orthographic projection to screen.
       Draw 12 edges. Colour edges by axis (X=red,Y=green,Z=blue
       for body-frame axes, length 1.5m).
       Update at simulation rate (no lag).

4.2  RIGHT PANEL — live telemetry plots (scrolling, last 30s):
       Plot 1: quaternion components q0..q3 vs time
       Plot 2: ω_x, ω_y, ω_z vs time (rad/s)
       Plot 3: ||q_err_vec|| vs time (log scale)
       Plot 4: ||τ_ctrl|| vs time with τ_max line
       Render these as pygame surfaces drawn with lines 
       (no matplotlib in real-time loop).

4.3  HUD overlay (top of screen):
       t = {time:.2f}s  | kp={kp:.2f}  kd={kd:.2f}
       ||qe|| = {err:.6f} | settled: {bool}
       Controller: ON/OFF | GravGrad: ON/OFF | Noise: ON/OFF
       MC status: {idle / running / done — N trials}

4.4  Screen layout: 1280×720, left 640px = 3D, right 640px = telemetry
     FPS: target 60, show actual FPS in HUD.

═══════════════════════════════════════════════════════════════════════
PART 5 — MONTE CARLO (montecarlo.py)
═══════════════════════════════════════════════════════════════════════

5.1  Run in background thread when M pressed.
     N=200 trials. Per trial sample:
       q0:  random unit quaternion (Shoemake method)
       ω0:  U([-0.5, 0.5]³) rad/s
       kp:  U([3.0, 7.0])
       kd:  U([7.0, 13.0])
     Each trial: simulate 60s at dt=0.005s WITH disturbances+noise.

5.2  Per trial record:
       settling_time: first t where ||q_err_vec|| < 0.01 
                      and remains < 0.01 for ≥5s. Else NaN.
       ss_error:      mean ||q_err_vec|| over final 10s
       max_torque:    max ||τ_ctrl|| over trial
       converged:     bool (not NaN)

5.3  On completion, print to stdout:
       ┌─────────────────────────────────────────────┐
       │ MONTE CARLO RESULTS — 200 trials            │
       ├─────────────────────────────────────────────┤
       │ Convergence rate:     XX.X%                 │
       │ Settling time:                              │
       │   mean ± std:         XX.Xs ± XX.Xs         │
       │   5th / 95th pct:     XX.Xs / XX.Xs         │
       │ Steady-state error:   X.XXXe-X ± X.XXXe-X  │
       │ Max torque (mean):    X.XXX N·m             │
       └─────────────────────────────────────────────┘

═══════════════════════════════════════════════════════════════════════
PART 6 — POST-RUN ANALYSIS (analysis.py)
═══════════════════════════════════════════════════════════════════════

Triggered by S key or --analysis-only flag. Produces 2 figures:

Figure 1 (nominal run, 2×2 subplot):
  [0,0] quaternion vs time
  [0,1] ω vs time
  [1,0] ||q_err|| log scale vs time, mark settling time
  [1,1] torque magnitude vs time, hline at τ_max

Figure 2 (MC results, if available, 2×2 subplot):
  [0,0] histogram of settling times (20 bins, mark mean)
  [0,1] scatter: kp vs settling_time, colour=kd, cmap=viridis
  [1,0] scatter: kd vs ss_error (log y)
  [1,1] CDF of settling times

Save both as ./output/nominal.png and ./output/montecarlo.png

═══════════════════════════════════════════════════════════════════════
PART 7 — VALIDATION ASSERTIONS
═══════════════════════════════════════════════════════════════════════

On startup, run and print PASS/FAIL for:
  V1: Torque-free sim 60s → max |ΔL| < 1e-6  (angular momentum)
  V2: ||q|| preservation  → max deviation < 1e-10 across 10k steps
  V3: RK4 order check     → halving dt reduces global error by ~16×
  V4: Quaternion multiply → q ⊗ q^{-1} = [1,0,0,0] to 1e-12

═══════════════════════════════════════════════════════════════════════
INITIAL CONDITIONS (defaults, all overridable via argparse)
═══════════════════════════════════════════════════════════════════════

  --q0        0.2 0.7 -0.5 0.3   (will be normalised)
  --w0        0.3 -0.2 0.1       (rad/s)
  --kp        5.0
  --kd        10.0
  --dt        0.005
  --duration  120.0              (seconds, 0 = run indefinitely)
  --target-quat 1 0 0 0
  --no-gui    run headless, output data to ./output/run_data.npz

═══════════════════════════════════════════════════════════════════════
DELIVERABLES CHECKLIST
═══════════════════════════════════════════════════════════════════════

□ All 7 files, fully implemented, no TODOs, no stubs
□ Runs with:  python main.py
□ Validation suite prints before GUI opens
□ Keyboard controls work in real-time without freezing sim
□ MC runs in background thread — GUI stays responsive
□ All plots saved correctly on S keypress
□ argparse --help shows all flags with descriptions
□ Every non-trivial line commented
□ No global mutable state outside dataclasses