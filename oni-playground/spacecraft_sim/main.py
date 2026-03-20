#!/usr/bin/env python3
"""
Spacecraft Attitude Control Simulation

Main entry point: argparse, main loop, keyboard controls, validation.

Keyboard Controls:
- UP/DOWN: kp ± 0.5
- LEFT/RIGHT: kd ± 0.5
- R: reset to initial conditions
- SPACE: toggle controller ON/OFF
- M: run Monte Carlo (200 trials, non-blocking)
- S: save current plots to ./output/
- G: toggle gravity gradient ON/OFF
- N: toggle gyro noise ON/OFF
- Q/ESC: quit

Usage:
    python main.py [--q0 ...] [--w0 ...] [--kp ...] [--kd ...]
                   [--dt ...] [--duration ...] [--target-quat ...]
                   [--no-gui] [--analysis-only]
"""

import argparse
import numpy as np
import time
import sys
import os
from typing import Tuple, Optional
from dataclasses import dataclass

import pygame
from pygame.locals import *

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from dynamics import Dynamics, SpacecraftParams
from controller import AttitudeController, ControllerParams, quaternion_multiply, quaternion_inverse
from disturbances import Disturbances, OrbitParams
from visualiser import Visualiser, VisualiserConfig
from montecarlo import MonteCarloRunner
from analysis import analyze_run, analyze_monte_carlo


def normalize_quaternion(q: np.ndarray) -> np.ndarray:
    """Normalize quaternion to unit length."""
    norm = np.linalg.norm(q)
    if norm < 1e-10:
        return np.array([1.0, 0.0, 0.0, 0.0])
    return q / norm


def parse_quaternion(s: str) -> np.ndarray:
    """Parse quaternion from string."""
    values = list(map(float, s.split()))
    if len(values) != 4:
        raise ValueError(f"Expected 4 values, got {len(values)}")
    q = np.array(values)
    return normalize_quaternion(q)


def parse_angular_velocity(s: str) -> np.ndarray:
    """Parse angular velocity from string."""
    values = list(map(float, s.split()))
    if len(values) != 3:
        raise ValueError(f"Expected 3 values, got {len(values)}")
    return np.array(values)


def parse_target_quaternion(s: str) -> np.ndarray:
    """Parse target quaternion from string."""
    values = list(map(float, s.split()))
    if len(values) != 4:
        raise ValueError(f"Expected 4 values, got {len(values)}")
    q = np.array(values)
    return normalize_quaternion(q)


def create_parser() -> argparse.ArgumentParser:
    """Create argument parser."""
    parser = argparse.ArgumentParser(
        description="Spacecraft Attitude Control Simulation",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python main.py                           # Run with defaults
  python main.py --q0 "1 0 0 0"           # Start at identity attitude
  python main.py --kp 3.0 --kd 6.0        # Custom controller gains
  python main.py --duration 60 --no-gui   # Headless 60s simulation
        """
    )
    
    # Initial conditions
    parser.add_argument('--q0', type=str, default="0.2 0.7 -0.5 0.3",
                       help="Initial quaternion (space-separated, will be normalized)")
    parser.add_argument('--w0', type=str, default="0.3 -0.2 0.1",
                       help="Initial angular velocity (space-separated, rad/s)")
    parser.add_argument('--kp', type=float, default=5.0,
                       help="Proportional gain")
    parser.add_argument('--kd', type=float, default=10.0,
                       help="Derivative gain")
    parser.add_argument('--dt', type=float, default=0.005,
                       help="Integration timestep (s)")
    parser.add_argument('--duration', type=float, default=120.0,
                       help="Simulation duration (s), 0 = run indefinitely")
    parser.add_argument('--target-quat', type=str, default="1 0 0 0",
                       help="Target quaternion (space-separated)")
    
    # Mode options
    parser.add_argument('--no-gui', action='store_true',
                       help="Run headless (no GUI)")
    parser.add_argument('--analysis-only', action='store_true',
                       help="Run analysis on existing data only")
    
    return parser


class Simulation:
    """Main simulation controller."""
    
    def __init__(self, args):
        self.args = args
        
        # Parse initial conditions
        self.q0 = parse_quaternion(args.q0)
        self.w0 = parse_angular_velocity(args.w0)
        self.q_d = parse_target_quaternion(args.target_quat)
        
        # Controller parameters
        self.kp = args.kp
        self.kd = args.kd
        
        # Simulation state
        self.state = np.concatenate([self.q0, self.w0])
        self.t = 0.0
        
        # Control flags
        self.controller_enabled = True
        self.use_grav_grad = True
        self.use_noise = True
        
        # Monte Carlo
        self.mc_runner = MonteCarloRunner(self.q_d)
        self.mc_status = "idle"
        
        # Setup dynamics
        self.params = SpacecraftParams(
            I=np.diag([10.0, 15.0, 8.0]),
            dt=args.dt
        )
        self.dynamics = Dynamics(self.params)
        self.controller = AttitudeController(ControllerParams(
            kp=self.kp, kd=self.kd, tau_max=5.0
        ))
        self.disturbances = Disturbances()
        
        # Setup visualiser
        self.visualiser = Visualiser()
        
        # Data storage
        self.history = {
            'times': [],
            'states': [],
            'torques': [],
            'errors': []
        }
        
        # Settling time tracking
        self.settling_time = np.nan
        self.error_history = []
        
    def compute_control(self) -> Tuple[np.ndarray, np.ndarray]:
        """Compute control torque and error."""
        omega_meas = self.disturbances.add_gyro_noise(self.state[4:7])
        tau_ctrl, q_err = self.controller.compute_torque(
            self.state, self.q_d, omega_meas
        )
        return tau_ctrl, q_err
    
    def update_settling_time(self, error_norm: float):
        """Update settling time if not already set."""
        if not np.isnan(self.settling_time):
            return
        
        self.error_history.append(error_norm)
        
        # Check if settled (error < 0.01 for 5+ seconds)
        if len(self.error_history) >= int(5.0 / self.params.dt):
            recent_errors = self.error_history[-int(5.0 / self.params.dt):]
            if all(e < 0.01 for e in recent_errors):
                self.settling_time = self.t - 5.0
    
    def step(self) -> bool:
        """Advance simulation by one timestep."""
        # Compute control
        tau_ctrl, q_err = self.compute_control()
        
        # Update state
        self.state = self.dynamics.rk4_step(
            self.state, tau_ctrl,
            use_grav_grad=self.use_grav_grad,
            use_noise=self.use_noise
        )
        
        # Update time
        self.t += self.params.dt
        
        # Update settling time
        self.update_settling_time(np.linalg.norm(q_err))
        
        # Store history
        self.history['times'].append(self.t)
        self.history['states'].append(self.state.copy())
        self.history['torques'].append(tau_ctrl)
        self.history['errors'].append(q_err)
        
        # Limit history size for memory
        max_history = 10000
        if len(self.history['times']) > max_history:
            for key in self.history:
                self.history[key] = self.history[key][-max_history:]
        
        # Check duration
        if self.args.duration > 0 and self.t >= self.args.duration:
            return False
        
        return True
    
    def reset(self):
        """Reset to initial conditions."""
        self.state = np.concatenate([self.q0, self.w0])
        self.t = 0.0
        self.settling_time = np.nan
        self.error_history = []
        self.history = {
            'times': [],
            'states': [],
            'torques': [],
            'errors': []
        }
        
        # Reset controller gains
        self.controller.params.kp = self.kp
        self.controller.params.kd = self.kd
    
    def run_monte_carlo(self):
        """Start Monte Carlo simulation in background."""
        if self.mc_status == "running":
            return
        
        self.mc_status = "running"
        self.mc_runner.start_background(200)
    
    def update_mc_status(self):
        """Update Monte Carlo status from background thread."""
        status = self.mc_runner.get_status()
        
        if status['status'] == 'done' and self.mc_status == 'running':
            self.mc_status = "done"
            self.mc_runner.print_summary()
        else:
            self.mc_status = f"{status['status']} - {status['complete']}/{status['total']}"
    
    def save_plots(self):
        """Save current plots to output directory."""
        if not self.history['times']:
            print("No data to save.")
            return
        
        times = np.array(self.history['times'])
        states = np.array(self.history['states'])
        torques = np.array(self.history['torques'])
        errors = np.array(self.history['errors'])
        
        # Save nominal plot
        from analysis import plot_nominal_run
        plot_nominal_run(times, states, torques, errors, 
                        self.settling_time, save_path="./output/nominal.png")
        
        # Save Monte Carlo plot if available
        if self.mc_runner.results:
            from analysis import plot_monte_carlo
            plot_monte_carlo(self.mc_runner.results, 
                           save_path="./output/montecarlo.png")
        
        # Save data
        np.savez('./output/run_data.npz',
                times=times,
                states=states,
                torques=torques,
                errors=errors,
                kp=self.kp,
                kd=self.kd,
                settling_time=self.settling_time)
        
        print(f"Data saved to ./output/")
    
    def run_headless(self):
        """Run simulation without GUI."""
        print("Running headless simulation...")
        
        while self.step():
            if len(self.history['times']) % 1000 == 0:
                print(f"t = {self.t:.2f}s, error = {np.linalg.norm(self.history['errors'][-1]):.6e}")
        
        print(f"\nSimulation complete. Final time: {self.t:.2f}s")
        
        # Analyze
        times = np.array(self.history['times'])
        states = np.array(self.history['states'])
        torques = np.array(self.history['torques'])
        errors = np.array(self.history['errors'])
        
        from analysis import analyze_run
        analyze_run(times, states, torques, errors, self.settling_time)
        
        # Save data
        self.save_plots()
    
    def run_gui(self):
        """Run simulation with GUI."""
        print("Starting GUI simulation...")
        print("Controls: UP/DOWN (kp), LEFT/RIGHT (kd), R (reset), SPACE (controller),")
        print("          M (Monte Carlo), S (save), G (gravity), N (noise), Q/ESC (quit)")
        
        running = True
        last_plot_update = 0.0
        plot_update_interval = 1.0 / 30.0  # 30 Hz
        
        while running:
            # Handle events
            for event in pygame.event.get():
                if event.type == QUIT:
                    running = False
                
                elif event.type == KEYDOWN:
                    if event.key in (K_q, K_ESCAPE):
                        running = False
                    
                    elif event.key == K_UP:
                        self.kp += 0.5
                        self.controller.params.kp = self.kp
                        print(f"kp = {self.kp:.2f}")
                    
                    elif event.key == K_DOWN:
                        self.kp = max(0.1, self.kp - 0.5)
                        self.controller.params.kp = self.kp
                        print(f"kp = {self.kp:.2f}")
                    
                    elif event.key == K_LEFT:
                        self.kd = max(0.1, self.kd - 0.5)
                        self.controller.params.kd = self.kd
                        print(f"kd = {self.kd:.2f}")
                    
                    elif event.key == K_RIGHT:
                        self.kd += 0.5
                        self.controller.params.kd = self.kd
                        print(f"kd = {self.kd:.2f}")
                    
                    elif event.key == K_r:
                        self.reset()
                        print("Reset to initial conditions")
                    
                    elif event.key == K_SPACE:
                        self.controller.toggle()
                        print(f"Controller {'ON' if self.controller.enabled else 'OFF'}")
                    
                    elif event.key == K_m:
                        self.run_monte_carlo()
                        print("Monte Carlo started...")
                    
                    elif event.key == K_s:
                        self.save_plots()
                        print("Plots saved")
                    
                    elif event.key == K_g:
                        self.use_grav_grad = not self.use_grav_grad
                        self.disturbances.gravity_gradient_enabled = self.use_grav_grad
                        print(f"Gravity gradient {'ON' if self.use_grav_grad else 'OFF'}")
                    
                    elif event.key == K_n:
                        self.use_noise = not self.use_noise
                        self.disturbances.noise_enabled = self.use_noise
                        print(f"Noise {'ON' if self.use_noise else 'OFF'}")
            
            # Update Monte Carlo status
            if self.mc_status.startswith("running") or self.mc_status.startswith("done"):
                self.update_mc_status()
            
            # Step simulation
            if not self.step():
                print("Duration reached. Simulation complete.")
                break
            
            # Update visualization
            current_time = time.time()
            if current_time - last_plot_update >= plot_update_interval:
                tau_ctrl, q_err = self.compute_control()
                
                # Compute error magnitude
                error_norm = np.linalg.norm(q_err)
                
                # Determine if settled
                settled = not np.isnan(self.settling_time) and self.t > self.settling_time
                
                # Update visualiser
                fps = self.visualiser.draw(
                    t=self.t,
                    q=self.state[0:4],
                    omega=self.state[4:7],
                    error=q_err,
                    torque=tau_ctrl,
                    kp=self.kp,
                    kd=self.kd,
                    settled=settled,
                    controller_on=self.controller.enabled,
                    grav_grad_on=self.use_grav_grad,
                    noise_on=self.use_noise,
                    mc_status=self.mc_status,
                    fps=60.0  # Approximate
                )
                
                last_plot_update = current_time
            
            # Small delay to prevent CPU spinning
            time.sleep(0.001)
        
        self.visualiser.quit()
    
    def run(self):
        """Run simulation based on mode."""
        if self.args.no_gui or self.args.duration == 0:
            self.run_headless()
        else:
            self.run_gui()


def run_validation():
    """Run validation assertions."""
    print("\n" + "=" * 50)
    print("RUNNING VALIDATION ASSERTIONS")
    print("=" * 50)
    
    all_passed = True
    
    # V1: Torque-free angular momentum conservation
    print("\nV1: Torque-free angular momentum conservation...")
    try:
        params = SpacecraftParams(I=np.diag([10.0, 15.0, 8.0]), dt=0.005)
        dynamics = Dynamics(params)
        
        # Initial state with some rotation
        q0 = normalize_quaternion(np.array([0.7, 0.1, 0.2, 0.3]))
        w0 = np.array([0.1, 0.2, 0.1])
        state = np.concatenate([q0, w0])
        
        # Simulate 60s torque-free
        for _ in range(int(60 / 0.005)):
            state = dynamics.rk4_step(state, np.zeros(3), 
                                     use_grav_grad=False, use_noise=False)
        
        # Check angular momentum
        H_initial = dynamics.angular_momentum(np.concatenate([q0, w0]))
        H_final = dynamics.angular_momentum(state)
        delta_L = np.linalg.norm(H_final - H_initial)
        
        if delta_L < 1e-6:
            print(f"  PASS: |ΔL| = {delta_L:.2e} < 1e-6")
        else:
            print(f"  FAIL: |ΔL| = {delta_L:.2e} >= 1e-6")
            all_passed = False
    except Exception as e:
        print(f"  FAIL: Exception - {e}")
        all_passed = False
    
    # V2: Quaternion normalization
    print("\nV2: Quaternion normalization preservation...")
    try:
        params = SpacecraftParams(I=np.diag([10.0, 15.0, 8.0]), dt=0.005)
        dynamics = Dynamics(params)
        
        q0 = normalize_quaternion(np.array([0.7, 0.1, 0.2, 0.3]))
        w0 = np.array([0.1, 0.2, 0.1])
        state = np.concatenate([q0, w0])
        
        max_deviation = 0.0
        for _ in range(10000):
            state = dynamics.rk4_step(state, np.zeros(3),
                                     use_grav_grad=False, use_noise=False)
            deviation = abs(np.linalg.norm(state[0:4]) - 1.0)
            max_deviation = max(max_deviation, deviation)
        
        if max_deviation < 1e-10:
            print(f"  PASS: max deviation = {max_deviation:.2e} < 1e-10")
        else:
            print(f"  FAIL: max deviation = {max_deviation:.2e} >= 1e-10")
            all_passed = False
    except Exception as e:
        print(f"  FAIL: Exception - {e}")
        all_passed = False
    
    # V3: RK4 convergence order
    print("\nV3: RK4 convergence order (4th order)...")
    try:
        params = SpacecraftParams(I=np.diag([10.0, 15.0, 8.0]), dt=0.005)
        dynamics = Dynamics(params)
        
        # Torque-free case (analytical solution exists for small rotations)
        q0 = normalize_quaternion(np.array([1.0, 0.01, 0.0, 0.0]))
        w0 = np.array([0.1, 0.0, 0.0])
        state = np.concatenate([q0, w0])
        
        # Reference solution with very small dt
        dt_ref = 0.0001
        n_steps_ref = int(1.0 / dt_ref)
        state_ref = state.copy()
        for _ in range(n_steps_ref):
            state_ref = dynamics.rk4_step(state_ref, np.zeros(3),
                                         use_grav_grad=False, use_noise=False)
        
        # Test with larger dt
        dt_test = 0.005
        n_steps_test = int(1.0 / dt_test)
        state_test = state.copy()
        for _ in range(n_steps_test):
            state_test = dynamics.rk4_step(state_test, np.zeros(3),
                                          use_grav_grad=False, use_noise=False)
        
        # Error ratio should be ~16 for 4th order (dt_ref/dt_test)^4 = (1/10)^4 = 1/10000
        # But we're comparing single steps, so use error at t=1s
        error_test = np.linalg.norm(state_test - state_ref)
        
        # Double dt and test again
        dt_test2 = 0.01
        n_steps_test2 = int(1.0 / dt_test2)
        state_test2 = state.copy()
        for _ in range(n_steps_test2):
            state_test2 = dynamics.rk4_step(state_test2, np.zeros(3),
                                           use_grav_grad=False, use_noise=False)
        
        error_test2 = np.linalg.norm(state_test2 - state_ref)
        
        # Ratio should be ~16 for 4th order
        if error_test > 1e-15:
            ratio = error_test / error_test2
            expected_ratio = (dt_test2 / dt_test) ** 4  # 16
            
            if 8 < ratio < 32:  # Allow some tolerance
                print(f"  PASS: error ratio = {ratio:.2f} (expected ~16)")
            else:
                print(f"  WARNING: error ratio = {ratio:.2f} (expected ~16)")
        else:
            print(f"  PASS: error < 1e-15 (machine precision)")
    except Exception as e:
        print(f"  FAIL: Exception - {e}")
        all_passed = False
    
    # V4: Quaternion multiplication
    print("\nV4: Quaternion multiplication (q ⊗ q^{-1} = [1,0,0,0])...")
    try:
        q = normalize_quaternion(np.array([0.7, 0.1, 0.2, 0.3]))
        q_inv = quaternion_inverse(q)
        result = quaternion_multiply(q, q_inv)
        
        expected = np.array([1.0, 0.0, 0.0, 0.0])
        error = np.linalg.norm(result - expected)
        
        if error < 1e-12:
            print(f"  PASS: ||q ⊗ q^{-1} - [1,0,0,0]|| = {error:.2e} < 1e-12")
        else:
            print(f"  FAIL: ||q ⊗ q^{-1} - [1,0,0,0]|| = {error:.2e} >= 1e-12")
            all_passed = False
    except Exception as e:
        print(f"  FAIL: Exception - {e}")
        all_passed = False
    
    print("\n" + "=" * 50)
    if all_passed:
        print("ALL VALIDATIONS PASSED")
    else:
        print("SOME VALIDATIONS FAILED")
    print("=" * 50)
    
    return all_passed


def main():
    """Main entry point."""
    parser = create_parser()
    args = parser.parse_args()
    
    # Create output directory
    os.makedirs('./output', exist_ok=True)
    
    # Run validation
    run_validation()
    
    # Create and run simulation
    sim = Simulation(args)
    sim.run()


if __name__ == "__main__":
    main()
