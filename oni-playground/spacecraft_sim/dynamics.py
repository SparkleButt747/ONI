"""
Dynamics module: RK4 integrator for spacecraft attitude dynamics.

Derivations:
------------
1. Euler's Rotational EOM (body frame):
   I·ω̇ = -ω × (I·ω) + τ_total
   
   Where:
   - I is the inertia tensor (diagonal for principal axes)
   - ω is the angular velocity vector in body frame
   - τ_total = τ_control + τ_disturbance
   
   Cross product ω × (I·ω) expands to:
   [ω_y*I_zz*ω_z - ω_z*I_yy*ω_y]
   [ω_z*I_xx*ω_x - ω_x*I_zz*ω_z]
   [ω_x*I_yy*ω_y - ω_y*I_xx*ω_x]

2. Quaternion Kinematics:
   q̇ = 0.5 · Ξ(q) · ω
   
   Where Ξ(q) is the 4×3 matrix:
   Ξ(q) = [ -q1  -q2  -q3 ]
          [  q0  -q3   q2 ]
          [  q3   q0  -q1 ]
          [ -q2   q1   q0 ]
   
   Scalar-first convention: q = [q0, q1, q2, q3]
   This ensures q̇ = 0.5 * q ⊗ [0, ω] (quaternion multiplication)

3. RK4 Integration:
   x_{n+1} = x_n + (dt/6) * (k1 + 2*k2 + 2*k3 + k4)
   where k1 = f(x_n), k2 = f(x_n + dt*k1/2), etc.
"""

import numpy as np
from dataclasses import dataclass
from typing import Tuple
from disturbances import Disturbances


@dataclass
class SpacecraftParams:
    """Spacecraft physical parameters."""
    I: np.ndarray  # Inertia tensor (3x3)
    dt: float      # Integration timestep


class Dynamics:
    """Spacecraft attitude dynamics with RK4 integration."""
    
    def __init__(self, params: SpacecraftParams):
        self.params = params
        self.disturbances = Disturbances()
        
    def cross_matrix(self, v: np.ndarray) -> np.ndarray:
        """Return skew-symmetric cross-product matrix."""
        return np.array([
            [0, -v[2], v[1]],
            [v[2], 0, -v[0]],
            [-v[1], v[0], 0]
        ])
    
    def angular_momentum(self, state: np.ndarray) -> np.ndarray:
        """Compute angular momentum H = I·ω."""
        omega = state[4:7]
        return self.params.I @ omega
    
    def torque_free_omega_dot(self, state: np.ndarray) -> np.ndarray:
        """
        Compute ω̇ for torque-free rotation.
        I·ω̇ = -ω × (I·ω)
        """
        omega = state[4:7]
        H = self.params.I @ omega
        torque = -np.cross(omega, H)
        return np.linalg.solve(self.params.I, torque)
    
    def compute_tau_total(self, state: np.ndarray, tau_control: np.ndarray,
                         use_grav_grad: bool, use_noise: bool) -> np.ndarray:
        """
        Compute total torque acting on spacecraft.
        τ_total = τ_control + τ_disturbance
        """
        tau_total = tau_control.copy()
        
        # Gravity-gradient torque
        if use_grav_grad:
            tau_gg = self.disturbances.gravity_gradient_torque(state)
            tau_total += tau_gg
            
        return tau_total
    
    def quaternion_kinematics(self, q: np.ndarray, omega: np.ndarray) -> np.ndarray:
        """
        Compute quaternion derivative: q̇ = 0.5 · Ξ(q) · ω
        
        Scalar-first: q = [q0, q1, q2, q3]
        """
        q0, q1, q2, q3 = q
        
        # Ξ(q) matrix for scalar-first convention
        Xi = np.array([
            [-q1, -q2, -q3],
            [ q0, -q3,  q2],
            [ q3,  q0, -q1],
            [-q2,  q1,  q0]
        ])
        
        return 0.5 * Xi @ omega
    
    def state_dot(self, state: np.ndarray, tau_control: np.ndarray,
                 use_grav_grad: bool, use_noise: bool) -> np.ndarray:
        """
        Compute full state derivative dx/dt.
        
        State: x = [q0, q1, q2, q3, wx, wy, wz]
        """
        q = state[0:4]
        omega = state[4:7]
        
        # Normalize quaternion to prevent drift
        q = q / np.linalg.norm(q)
        
        # Compute total torque
        tau_total = self.compute_tau_total(state, tau_control, use_grav_grad, use_noise)
        
        # Euler's equation: I·ω̇ = -ω × (I·ω) + τ_total
        H = self.params.I @ omega
        omega_dot = np.linalg.solve(self.params.I, -np.cross(omega, H) + tau_total)
        
        # Quaternion kinematics: q̇ = 0.5 · Ξ(q) · ω
        q_dot = self.quaternion_kinematics(q, omega)
        
        return np.concatenate([q_dot, omega_dot])
    
    def rk4_step(self, state: np.ndarray, tau_control: np.ndarray,
                use_grav_grad: bool, use_noise: bool) -> np.ndarray:
        """
        Perform one RK4 integration step.
        
        x_{n+1} = x_n + (dt/6) * (k1 + 2*k2 + 2*k3 + k4)
        """
        dt = self.params.dt
        
        # k1 = f(x_n)
        k1 = self.state_dot(state, tau_control, use_grav_grad, use_noise)
        
        # k2 = f(x_n + dt*k1/2)
        state2 = state + dt * k1 / 2
        state2[0:4] = state2[0:4] / np.linalg.norm(state2[0:4])  # Renormalize
        k2 = self.state_dot(state2, tau_control, use_grav_grad, use_noise)
        
        # k3 = f(x_n + dt*k2/2)
        state3 = state + dt * k2 / 2
        state3[0:4] = state3[0:4] / np.linalg.norm(state3[0:4])  # Renormalize
        k3 = self.state_dot(state3, tau_control, use_grav_grad, use_noise)
        
        # k4 = f(x_n + dt*k3)
        state4 = state + dt * k3
        state4[0:4] = state4[0:4] / np.linalg.norm(state4[0:4])  # Renormalize
        k4 = self.state_dot(state4, tau_control, use_grav_grad, use_noise)
        
        # Combine
        state_new = state + (dt / 6) * (k1 + 2*k2 + 2*k3 + k4)
        
        # Final quaternion normalization
        state_new[0:4] = state_new[0:4] / np.linalg.norm(state_new[0:4])
        
        return state_new
    
    def simulate(self, state_init: np.ndarray, tau_control_func,
                use_grav_grad: bool, use_noise: bool,
                duration: float) -> Tuple[np.ndarray, np.ndarray, np.ndarray]:
        """
        Simulate spacecraft dynamics for given duration.
        
        Returns:
            times: array of time points
            states: array of state vectors [N, 7]
            torques: array of control torques [N, 3]
        """
        n_steps = int(duration / self.params.dt) + 1
        times = np.linspace(0, duration, n_steps)
        states = np.zeros((n_steps, 7))
        torques = np.zeros((n_steps, 3))
        
        states[0] = state_init
        
        for i in range(1, n_steps):
            tau_control = tau_control_func(states[i-1])
            torques[i-1] = tau_control
            states[i] = self.rk4_step(states[i-1], tau_control, 
                                     use_grav_grad, use_noise)
        
        # Final torque computation
        torques[-1] = tau_control_func(states[-1])
        
        return times, states, torques
