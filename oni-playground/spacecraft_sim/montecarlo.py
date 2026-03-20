"""
Monte Carlo module: Background thread for Monte Carlo simulation.

Runs 200 trials with randomized initial conditions and controller gains.
Each trial simulates 60s with disturbances and noise.

Per trial records:
- settling_time: first t where ||q_err|| < 0.01 for ≥5s
- ss_error: mean ||q_err|| over final 10s
- max_torque: max ||τ_ctrl|| over trial
- converged: bool (not NaN)

Output: Statistics summary to stdout.
"""

import numpy as np
import threading
import time
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, field
from controller import AttitudeController, ControllerParams
from dynamics import Dynamics, SpacecraftParams
from disturbances import Disturbances, OrbitParams
import random


def random_unit_quaternion() -> np.ndarray:
    """
    Generate random unit quaternion using Shoemake method.
    
    This ensures uniform distribution over SO(3).
    """
    u1, u2, u3 = np.random.random(3)
    
    q0 = np.sqrt(1 - u1) * np.sin(2 * np.pi * u2)
    q1 = np.sqrt(1 - u1) * np.cos(2 * np.pi * u2)
    q2 = np.sqrt(u1) * np.sin(2 * np.pi * u3)
    q3 = np.sqrt(u1) * np.cos(2 * np.pi * u3)
    
    q = np.array([q0, q1, q2, q3])
    return q / np.linalg.norm(q)


def random_angular_velocity() -> np.ndarray:
    """Generate random angular velocity in [-0.5, 0.5]³ rad/s."""
    return np.random.uniform(-0.5, 0.5, 3)


@dataclass
class TrialResult:
    """Results from a single Monte Carlo trial."""
    settling_time: float
    ss_error: float
    max_torque: float
    converged: bool
    kp: float
    kd: float


class MonteCarloRunner:
    """Runs Monte Carlo simulations in background thread."""
    
    def __init__(self, target_quat: np.ndarray = None):
        self.target_quat = target_quat if target_quat is not None else np.array([1.0, 0.0, 0.0, 0.0])
        self.params = SpacecraftParams(
            I=np.diag([10.0, 15.0, 8.0]),
            dt=0.005
        )
        self.dynamics = Dynamics(self.params)
        self.disturbances = Disturbances()
        
        self.running = False
        self.trials_complete = 0
        self.total_trials = 200
        self.results: List[TrialResult] = []
        self.thread: Optional[threading.Thread] = None
        
    def run_trial(self, kp: float, kd: float, q0: np.ndarray, 
                 w0: np.ndarray) -> TrialResult:
        """Run a single Monte Carlo trial."""
        # Initialize controller
        ctrl_params = ControllerParams(kp=kp, kd=kd, tau_max=5.0)
        controller = AttitudeController(ctrl_params)
        
        # Initial state
        state = np.concatenate([q0, w0])
        
        # Simulation parameters
        duration = 60.0
        n_steps = int(duration / self.params.dt) + 1
        
        # Storage
        errors = []
        torques = []
        times = np.linspace(0, duration, n_steps)
        
        # Run simulation
        for i in range(1, n_steps):
            # Compute control torque
            omega_meas = self.disturbances.add_gyro_noise(state[4:7])
            tau_ctrl, q_err = controller.compute_torque(state, self.target_quat, omega_meas)
            
            # Store data
            errors.append(np.linalg.norm(q_err))
            torques.append(np.linalg.norm(tau_ctrl))
            
            # Update state
            state = self.dynamics.rk4_step(state, tau_ctrl, 
                                          use_grav_grad=True, 
                                          use_noise=True)
        
        # Final torque
        omega_meas = self.disturbances.add_gyro_noise(state[4:7])
        tau_ctrl, q_err = controller.compute_torque(state, self.target_quat, omega_meas)
        torques.append(np.linalg.norm(tau_ctrl))
        
        # Compute metrics
        error_array = np.array(errors)
        torque_array = np.array(torques)
        
        # Settling time: first t where ||q_err|| < 0.01 for ≥5s
        settling_time = np.nan
        window_size = int(5.0 / 0.005)  # 5 seconds worth of steps
        
        for i in range(len(error_array) - window_size):
            if np.all(error_array[i:i+window_size] < 0.01):
                settling_time = times[i]
                break
        
        # Steady-state error: mean over final 10s
        final_window = int(10.0 / 0.005)
        ss_error = np.mean(error_array[-final_window:]) if len(error_array) >= final_window else np.mean(error_array)
        
        # Max torque
        max_torque = np.max(torque_array)
        
        # Converged: not NaN
        converged = not np.isnan(settling_time)
        
        return TrialResult(
            settling_time=settling_time,
            ss_error=ss_error,
            max_torque=max_torque,
            converged=converged,
            kp=kp,
            kd=kd
        )
    
    def run(self, n_trials: int = 200) -> List[TrialResult]:
        """Run Monte Carlo simulation."""
        self.running = True
        self.trials_complete = 0
        self.total_trials = n_trials
        self.results = []
        
        for i in range(n_trials):
            if not self.running:
                break
            
            # Sample parameters
            q0 = random_unit_quaternion()
            w0 = random_angular_velocity()
            kp = np.random.uniform(3.0, 7.0)
            kd = np.random.uniform(7.0, 13.0)
            
            # Run trial
            result = self.run_trial(kp, kd, q0, w0)
            self.results.append(result)
            self.trials_complete += 1
        
        self.running = False
        return self.results
    
    def start_background(self, n_trials: int = 200):
        """Start Monte Carlo in background thread."""
        if self.thread is not None and self.thread.is_alive():
            return  # Already running
        
        self.thread = threading.Thread(
            target=self.run,
            args=(n_trials,),
            daemon=True
        )
        self.thread.start()
    
    def get_status(self) -> Dict:
        """Get current Monte Carlo status."""
        if not self.results:
            return {
                'status': 'idle',
                'complete': 0,
                'total': self.total_trials,
                'converged': 0
            }
        
        converged = sum(1 for r in self.results if r.converged)
        
        if self.running:
            status = 'running'
        else:
            status = 'done'
        
        return {
            'status': status,
            'complete': len(self.results),
            'total': self.total_trials,
            'converged': converged
        }
    
    def print_summary(self):
        """Print Monte Carlo results summary."""
        if not self.results:
            print("No Monte Carlo results available.")
            return
        
        results = self.results
        
        # Convergence rate
        converged = sum(1 for r in results if r.converged)
        conv_rate = 100 * converged / len(results)
        
        # Settling times
        settling_times = [r.settling_time for r in results if not np.isnan(r.settling_time)]
        
        if settling_times:
            st_mean = np.mean(settling_times)
            st_std = np.std(settling_times)
            st_5th = np.percentile(settling_times, 5)
            st_95th = np.percentile(settling_times, 95)
        else:
            st_mean = st_std = st_5th = st_95th = np.nan
        
        # Steady-state error
        ss_errors = [r.ss_error for r in results]
        ss_mean = np.mean(ss_errors)
        ss_std = np.std(ss_errors)
        
        # Max torque
        max_torques = [r.max_torque for r in results]
        mt_mean = np.mean(max_torques)
        
        # Print summary
        print("\n" + "┌" + "─" * 47 + "┐")
        print("│" + " MONTE CARLO RESULTS — {} trials".format(len(results)).ljust(48) + "│")
        print("├" + "─" * 47 + "┤")
        print("│ Convergence rate:     {:6.1f}%".format(conv_rate).ljust(48) + "│")
        print("│ Settling time:".ljust(48) + "│")
        print("│   mean ± std:         {:6.1f}s ± {:6.1f}s".format(st_mean, st_std).ljust(48) + "│")
        print("│   5th / 95th pct:     {:6.1f}s / {:6.1f}s".format(st_5th, st_95th).ljust(48) + "│")
        print("│ Steady-state error:   {:.3e} ± {:.3e}".format(ss_mean, ss_std).ljust(48) + "│")
        print("│ Max torque (mean):    {:6.3f} N·m".format(mt_mean).ljust(48) + "│")
        print("└" + "─" * 47 + "┘")


def run_monte_carlo(target_quat: np.ndarray = None, n_trials: int = 200) -> MonteCarloRunner:
    """Convenience function to run Monte Carlo and return runner."""
    runner = MonteCarloRunner(target_quat)
    runner.run(n_trials)
    runner.print_summary()
    return runner
