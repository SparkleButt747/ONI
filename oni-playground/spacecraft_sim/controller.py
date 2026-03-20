"""
Controller module: Lyapunov-based quaternion error controller.

Control Law:
------------
For attitude tracking with quaternion q_d (desired) and current q:

1. Quaternion error:
   q_err = q_d^{-1} ⊗ q
   
   This represents the rotation from current attitude to desired attitude.

2. Error vector (axis-angle representation):
   q_err_vec = [q_err1, q_err2, q_err3]
   
   For small errors, ||q_err_vec|| ≈ ||θ||/2 where θ is the rotation angle.

3. Control torque (Lyapunov-based):
   τ_ctrl = -kp · q_err_vec - kd · ω_meas
   
   This is derived from the Lyapunov function:
   V = ||q_err_vec||² + (1/kp) · ||ω_meas||²
   
   The control law ensures V̇ < 0, guaranteeing asymptotic stability.

4. Actuator saturation:
   If ||τ_ctrl|| > τ_max, scale τ_ctrl *= τ_max / ||τ_ctrl||
   This preserves direction while limiting magnitude.
"""

import numpy as np
from dataclasses import dataclass
from typing import Callable
from scipy.spatial.transform import Rotation as R


def quaternion_multiply(q1: np.ndarray, q2: np.ndarray) -> np.ndarray:
    """
    Multiply two quaternions.
    
    Scalar-first convention: q = [q0, q1, q2, q3]
    q1 ⊗ q2 = [q10*q20 - q1·q2, q10*q2 + q20*q1 + q1×q2]
    """
    q10, q1v = q1[0], q1[1:4]
    q20, q2v = q2[0], q2[1:4]
    
    scalar = q10 * q20 - np.dot(q1v, q2v)
    vector = q10 * q2v + q20 * q1v + np.cross(q1v, q2v)
    
    return np.concatenate([[scalar], vector])


def quaternion_inverse(q: np.ndarray) -> np.ndarray:
    """
    Compute quaternion inverse.
    
    q^{-1} = [q0, -q1, -q2, -q3] for unit quaternion
    """
    return np.array([q[0], -q[1], -q[2], -q[3]])


def quaternion_to_rotation_matrix(q: np.ndarray) -> np.ndarray:
    """
    Convert quaternion to rotation matrix.
    
    Scalar-first: q = [q0, q1, q2, q3]
    """
    q0, q1, q2, q3 = q
    return np.array([
        [1 - 2*(q2**2 + q3**2), 2*(q1*q2 - q0*q3), 2*(q1*q3 + q0*q2)],
        [2*(q1*q2 + q0*q3), 1 - 2*(q1**2 + q3**2), 2*(q2*q3 - q0*q1)],
        [2*(q1*q3 - q0*q2), 2*(q2*q3 + q0*q1), 1 - 2*(q1**2 + q2**2)]
    ])


@dataclass
class ControllerParams:
    """Controller parameters."""
    kp: float = 5.0      # Proportional gain
    kd: float = 10.0     # Derivative gain
    tau_max: float = 5.0 # Maximum torque magnitude (N·m)


class AttitudeController:
    """Lyapunov-based quaternion error controller."""
    
    def __init__(self, params: ControllerParams = ControllerParams()):
        self.params = params
        self.enabled = True
        
    def compute_error(self, q: np.ndarray, q_d: np.ndarray) -> np.ndarray:
        """
        Compute quaternion error: q_err = q_d^{-1} ⊗ q
        
        Returns the error vector q_err[1:4].
        """
        q_d_inv = quaternion_inverse(q_d)
        q_err = quaternion_multiply(q_d_inv, q)
        return q_err[1:4]  # Vector part
    
    def compute_torque(self, state: np.ndarray, q_d: np.ndarray,
                      omega_meas: np.ndarray) -> np.ndarray:
        """
        Compute control torque using Lyapunov-based law.
        
        τ_ctrl = -kp · q_err_vec - kd · ω_meas
        """
        if not self.enabled:
            return np.zeros(3)
        
        q = state[0:4]
        omega = state[4:7]
        
        # Compute quaternion error
        q_err_vec = self.compute_error(q, q_d)
        
        # Control law
        tau_ctrl = -self.params.kp * q_err_vec - self.params.kd * omega_meas
        
        # Actuator saturation
        tau_norm = np.linalg.norm(tau_ctrl)
        if tau_norm > self.params.tau_max:
            tau_ctrl = tau_ctrl * self.params.tau_max / tau_norm
        
        return tau_ctrl, q_err_vec
    
    def toggle(self):
        """Toggle controller on/off."""
        self.enabled = not self.enabled


def create_controller_from_args(args) -> AttitudeController:
    """Create controller from argparse arguments."""
    params = ControllerParams(
        kp=args.kp,
        kd=args.kd,
        tau_max=5.0
    )
    return AttitudeController(params)
