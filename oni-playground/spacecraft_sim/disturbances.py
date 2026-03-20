"""
Disturbances module: Gravity-gradient torque and sensor noise.

1. Gravity-Gradient Torque:
   τ_gg = (3μ/r³) · n̂ × (I · n̂)
   
   Where:
   - μ = 3.986e14 m³/s² (Earth's gravitational parameter)
   - r = 6.771e6 m (orbital radius, ~400 km altitude)
   - n̂ = [0, 0, 1] is the body-frame nadir vector (pointing to Earth)
   
   This torque arises because the spacecraft's center of mass is in orbit,
   but different parts are at slightly different distances from Earth,
   creating a net torque that tries to align the spacecraft's longest
   axis with the nadir direction.

2. Gyroscope Noise:
   ω_meas = ω_true + η
   
   Where η ~ N(0, σ²·I₃) with σ = 0.01 rad/s.
   This models realistic sensor noise affecting the controller's feedback.
"""

import numpy as np
from dataclasses import dataclass
from typing import Tuple


@dataclass
class OrbitParams:
    """Orbital parameters for gravity-gradient computation."""
    mu: float = 3.986e14      # Earth's gravitational parameter (m³/s²)
    r: float = 6.771e6        # Orbital radius (m), ~400 km altitude


class Disturbances:
    """Compute environmental disturbance torques."""
    
    def __init__(self, orbit_params: OrbitParams = OrbitParams()):
        self.orbit = orbit_params
        self.noise_enabled = True
        self.gravity_gradient_enabled = True
        
        # Precompute constant factor
        self.gg_factor = 3 * self.orbit.mu / (self.orbit.r ** 3)
        
    def gravity_gradient_torque(self, state: np.ndarray) -> np.ndarray:
        """
        Compute gravity-gradient torque in body frame.
        
        τ_gg = (3μ/r³) · n̂ × (I · n̂)
        
        Where n̂ = [0, 0, 1] is the nadir vector in body frame.
        """
        if not self.gravity_gradient_enabled:
            return np.zeros(3)
        
        # Nadir vector in body frame (z-axis points to Earth)
        n_hat = np.array([0.0, 0.0, 1.0])
        
        # Transform to body frame using current attitude
        q = state[0:4]
        R_body_to_inertial = self._quaternion_to_rotation_matrix(q)
        
        # n̂ in body frame: rotate inertial nadir to body frame
        # n_body = R^T @ n_inertial, where n_inertial = [0,0,1]
        n_body = R_body_to_inertial.T @ n_hat
        
        # Compute torque: τ = (3μ/r³) * n × (I·n)
        H = self._inertia_tensor() @ n_body
        tau_gg = self.gg_factor * np.cross(n_body, H)
        
        return tau_gg
    
    def _inertia_tensor(self) -> np.ndarray:
        """Return spacecraft inertia tensor."""
        return np.diag([10.0, 15.0, 8.0])  # kg·m²
    
    def _quaternion_to_rotation_matrix(self, q: np.ndarray) -> np.ndarray:
        """Convert quaternion to rotation matrix."""
        q0, q1, q2, q3 = q
        return np.array([
            [1 - 2*(q2**2 + q3**2), 2*(q1*q2 - q0*q3), 2*(q1*q3 + q0*q2)],
            [2*(q1*q2 + q0*q3), 1 - 2*(q1**2 + q3**2), 2*(q2*q3 - q0*q1)],
            [2*(q1*q3 - q0*q2), 2*(q2*q3 + q0*q1), 1 - 2*(q1**2 + q2**2)]
        ])
    
    def add_gyro_noise(self, omega_true: np.ndarray) -> np.ndarray:
        """
        Add gyroscope noise to true angular velocity.
        
        ω_meas = ω_true + η, η ~ N(0, (0.01)²·I₃)
        """
        if not self.noise_enabled:
            return omega_true
        
        sigma = 0.01  # rad/s
        noise = np.random.normal(0, sigma, 3)
        return omega_true + noise
    
    def toggle_gravity_gradient(self):
        """Toggle gravity-gradient torque on/off."""
        self.gravity_gradient_enabled = not self.gravity_gradient_enabled
    
    def toggle_noise(self):
        """Toggle gyroscope noise on/off."""
        self.noise_enabled = not self.noise_enabled
