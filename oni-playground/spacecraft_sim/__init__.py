"""
Spacecraft Attitude Control Simulation Package

This package provides a comprehensive simulation environment for spacecraft
attitude dynamics and control, including:

- Physics-based dynamics with gravity gradient and disturbance torques
- PID attitude controller with quaternion-based control
- Monte Carlo analysis for robustness evaluation
- Real-time visualization with Pygame
- Comprehensive validation and analysis tools
"""

from .dynamics import Dynamics, SpacecraftParams
from .controller import AttitudeController, ControllerParams
from .disturbances import Disturbances, OrbitParams
from .visualiser import Visualiser, VisualiserConfig
from .montecarlo import MonteCarloRunner
from .analysis import analyze_run, analyze_monte_carlo, plot_nominal_run, plot_monte_carlo

__version__ = "1.0.0"
__author__ = "Spacecraft Control Team"

__all__ = [
    'Dynamics',
    'SpacecraftParams',
    'AttitudeController', 
    'ControllerParams',
    'Disturbances',
    'OrbitParams',
    'Visualiser',
    'VisualiserConfig',
    'MonteCarloRunner',
    'analyze_run',
    'analyze_monte_carlo',
    'plot_nominal_run',
    'plot_monte_carlo',
]
