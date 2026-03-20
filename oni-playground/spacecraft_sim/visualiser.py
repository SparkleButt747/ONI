"""
Visualiser module: Pygame 3D wireframe and telemetry display.

1. 3D Wireframe Spacecraft:
   - Box: 2m × 1.5m × 1m
   - 8 vertices in body frame
   - Rotate using current quaternion
   - Orthographic projection to screen
   - 12 edges, colored by axis (X=red, Y=green, Z=blue)

2. Telemetry Plots (real-time, scrolling 30s):
   - Plot 1: q0..q3 vs time
   - Plot 2: ω_x, ω_y, ω_z vs time
   - Plot 3: ||q_err_vec|| vs time (log scale)
   - Plot 4: ||τ_ctrl|| vs time with τ_max line

3. HUD Overlay:
   - Time, gains, error, status indicators
   - FPS counter
"""

import pygame
import numpy as np
from typing import Tuple, List
from dataclasses import dataclass
from controller import quaternion_to_rotation_matrix


@dataclass
class VisualiserConfig:
    """Visualiser configuration."""
    screen_width: int = 1280
    screen_height: int = 720
    left_panel_width: int = 640
    plot_history_seconds: float = 30.0
    plot_update_rate: int = 30  # Hz
    fps_target: int = 60


class Visualiser:
    """Pygame visualiser for spacecraft simulation."""
    
    def __init__(self, config: VisualiserConfig = VisualiserConfig()):
        pygame.init()
        self.config = config
        self.screen = pygame.display.set_mode(
            (config.screen_width, config.screen_height)
        )
        pygame.display.set_caption("Spacecraft Attitude Control Simulation")
        
        # Spacecraft geometry (body frame, meters)
        half_dims = np.array([1.0, 0.75, 0.5])  # Half of 2×1.5×1
        self.vertices_body = np.array([
            [-half_dims[0], -half_dims[1], -half_dims[2]],  # 0: back-bottom-left
            [ half_dims[0], -half_dims[1], -half_dims[2]],  # 1: back-bottom-right
            [ half_dims[0],  half_dims[1], -half_dims[2]],  # 2: back-top-right
            [-half_dims[0],  half_dims[1], -half_dims[2]],  # 3: back-top-left
            [-half_dims[0], -half_dims[1],  half_dims[2]],  # 4: front-bottom-left
            [ half_dims[0], -half_dims[1],  half_dims[2]],  # 5: front-bottom-right
            [ half_dims[0],  half_dims[1],  half_dims[2]],  # 6: front-top-right
            [-half_dims[0],  half_dims[1],  half_dims[2]],  # 7: front-top-left
        ])
        
        # Edges: pairs of vertex indices
        self.edges = [
            (0, 1), (1, 2), (2, 3), (3, 0),  # Back face
            (4, 5), (5, 6), (6, 7), (7, 4),  # Front face
            (0, 4), (1, 5), (2, 6), (3, 7),  # Connecting edges
        ]
        
        # Axis colors (RGB)
        self.axis_colors = [
            (255, 0, 0),   # X: red
            (0, 255, 0),   # Y: green
            (0, 0, 255),   # Z: blue
        ]
        
        # Axis length for visualization
        self.axis_length = 1.5
        
        # Plot data storage
        self.plot_data = {
            'time': [],
            'q': [],
            'omega': [],
            'error': [],
            'torque': [],
        }
        
        # Fonts
        self.font_small = pygame.font.Font(None, 20)
        self.font_large = pygame.font.Font(None, 24)
        
        # Clock for FPS
        self.clock = pygame.time.Clock()
        
    def world_to_screen(self, point: np.ndarray, 
                       camera_dist: float = 8.0) -> Tuple[int, int]:
        """
        Transform world point to screen coordinates.
        
        Uses orthographic projection with simple perspective scaling.
        """
        # Simple perspective projection
        scale = camera_dist / (camera_dist + point[2])
        x = int(point[0] * scale * 50 + self.config.left_panel_width / 2)
        y = int(-point[1] * scale * 50 + self.config.screen_height / 2)
        return x, y
    
    def rotate_vertices(self, q: np.ndarray) -> np.ndarray:
        """Rotate vertices from body frame to world frame."""
        R_body_to_inertial = quaternion_to_rotation_matrix(q)
        return self.vertices_body @ R_body_to_inertial.T
    
    def draw_spacecraft(self, q: np.ndarray):
        """Draw 3D wireframe spacecraft."""
        # Rotate vertices
        vertices_world = self.rotate_vertices(q)
        
        # Draw edges
        for edge in self.edges:
            p1 = self.world_to_screen(vertices_world[edge[0]])
            p2 = self.world_to_screen(vertices_world[edge[1]])
            pygame.draw.line(self.screen, (200, 200, 200), p1, p2, 2)
        
        # Draw body axes
        origin = np.array([0, 0, 0])
        axes_end = np.array([
            [self.axis_length, 0, 0],
            [0, self.axis_length, 0],
            [0, 0, self.axis_length],
        ])
        
        # Transform axes to world frame
        R_body_to_inertial = quaternion_to_rotation_matrix(q)
        axes_world = axes_end @ R_body_to_inertial.T
        
        for i, axis_end in enumerate(axes_world):
            p_start = self.world_to_screen(origin)
            p_end = self.world_to_screen(axis_end)
            pygame.draw.line(self.screen, self.axis_colors[i], p_start, p_end, 3)
        
    def create_plot_surface(self, data: List[float], 
                           colors: List[Tuple[int, int, int]],
                           labels: List[str],
                           y_label: str,
                           max_time: float = 30.0,
                           log_scale: bool = False) -> pygame.Surface:
        """Create a plot surface for telemetry data."""
        width = 600
        height = 150
        
        surf = pygame.Surface((width, height))
        surf.fill((30, 30, 30))
        
        if len(data) < 2:
            text = self.font_small.render("No data", True, (100, 100, 100))
            surf.blit(text, (10, height // 2))
            return surf
        
        # Get time range
        times = np.linspace(0, max_time, len(data))
        
        # Find min/max for scaling
        if log_scale:
            valid_data = [d for d in data if d > 0]
            if not valid_data:
                text = self.font_small.render("No valid data", True, (100, 100, 100))
                surf.blit(text, (10, height // 2))
                return surf
            min_val = min(valid_data) * 0.5
            max_val = max(valid_data) * 2
        else:
            min_val = min(data) * 1.1 if min(data) < 0 else min(0, min(data) * 1.1)
            max_val = max(data) * 1.1 if max(data) > 0 else max(0, max(data) * 1.1)
        
        if max_val - min_val < 1e-10:
            max_val = min_val + 1e-10
        
        # Draw grid
        pygame.draw.line(surf, (50, 50, 50), (50, height - 30), 
                        (width - 20, height - 30), 1)  # X-axis
        pygame.draw.line(surf, (50, 50, 50), (50, 10), 
                        (50, height - 30), 1)  # Y-axis
        
        # Plot each data series
        for i, series_data in enumerate(data if isinstance(data[0], (list, tuple)) else [data]):
            if not isinstance(data[0], (list, tuple)):
                series_data = data
            
            # Resample to fit width
            n_points = min(width - 70, len(series_data))
            if n_points < 2:
                continue
                
            indices = np.linspace(0, len(series_data) - 1, n_points, dtype=int)
            plot_points = []
            
            for j, idx in enumerate(indices):
                t = times[idx]
                v = series_data[idx]
                
                x = 50 + (t / max_time) * (width - 70)
                if log_scale and v > 0:
                    y = height - 30 - ((np.log10(v) - np.log10(min_val)) / 
                                      (np.log10(max_val) - np.log10(min_val))) * (height - 40)
                elif not log_scale:
                    y = height - 30 - ((v - min_val) / (max_val - min_val)) * (height - 40)
                else:
                    y = height - 30  # Invalid for log scale
                
                x = max(50, min(width - 20, x))
                y = max(10, min(height - 30, y))
                plot_points.append((int(x), int(y)))
            
            if len(plot_points) > 1:
                pygame.draw.lines(surf, colors[i % len(colors)], False, 
                                plot_points, 2)
        
        # Draw labels
        text = self.font_small.render(y_label, True, (200, 200, 200))
        surf.blit(text, (10, 5))
        
        # Legend
        for i, label in enumerate(labels[:3]):
            color = colors[i % len(colors)]
            pygame.draw.circle(surf, color, (width - 80, 15 + i * 15), 3)
            text = self.font_small.render(label, True, (200, 200, 200))
            surf.blit(text, (width - 70, 10 + i * 15))
        
        return surf
    
    def update_plot_data(self, t: float, q: np.ndarray, omega: np.ndarray,
                        error: np.ndarray, torque: np.ndarray):
        """Update plot data buffers."""
        self.plot_data['time'].append(t)
        self.plot_data['q'].append(q.copy())
        self.plot_data['omega'].append(omega.copy())
        self.plot_data['error'].append(np.linalg.norm(error))
        self.plot_data['torque'].append(np.linalg.norm(torque))
        
        # Keep only recent data
        max_points = int(self.config.plot_history_seconds / 0.005)
        for key in self.plot_data:
            while len(self.plot_data[key]) > max_points:
                self.plot_data[key].pop(0)
    
    def draw_telemetry(self):
        """Draw telemetry plots on right panel."""
        x_offset = self.config.left_panel_width
        
        # Extract data
        time_data = self.plot_data['time']
        q_data = self.plot_data['q']
        omega_data = self.plot_data['omega']
        error_data = self.plot_data['error']
        torque_data = self.plot_data['torque']
        
        if not time_data:
            return
        
        # Plot 1: Quaternions
        q_surfaces = []
        q_colors = [(255, 255, 255), (255, 100, 100), (100, 255, 100), (100, 100, 255)]
        q_labels = ['q0', 'q1', 'q2', 'q3']
        
        if q_data:
            q_array = np.array(q_data)
            for i in range(4):
                q_surfaces.append(self.create_plot_surface(
                    q_array[:, i].tolist(),
                    [q_colors[i]],
                    [q_labels[i]],
                    "Quaternion",
                    log_scale=False
                ))
        
        # Plot 2: Angular velocity
        omega_surfaces = []
        omega_colors = [(255, 100, 100), (100, 255, 100), (100, 100, 255)]
        omega_labels = ['wx', 'wy', 'wz']
        
        if omega_data:
            omega_array = np.array(omega_data)
            for i in range(3):
                omega_surfaces.append(self.create_plot_surface(
                    omega_array[:, i].tolist(),
                    [omega_colors[i]],
                    [omega_labels[i]],
                    "Angular Velocity (rad/s)",
                    log_scale=False
                ))
        
        # Plot 3: Error magnitude (log scale)
        error_surface = self.create_plot_surface(
            error_data,
            [(255, 255, 0)],
            ['||q_err||'],
            "Error Magnitude (log)",
            log_scale=True
        )
        
        # Plot 4: Torque magnitude
        torque_surface = self.create_plot_surface(
            torque_data,
            [(255, 100, 100)],
            ['||τ_ctrl||'],
            "Torque Magnitude (N·m)",
            log_scale=False
        )
        
        # Draw plots in 2×2 grid
        plot_height = 140
        plot_width = 600
        
        # Row 1
        if q_surfaces:
            self.screen.blit(q_surfaces[0], (x_offset + 20, 20))
            self.screen.blit(q_surfaces[1], (x_offset + 20 + plot_width//2 + 10, 20))
        
        # Row 2
        if omega_surfaces:
            self.screen.blit(omega_surfaces[0], (x_offset + 20, 20 + plot_height + 10))
            self.screen.blit(omega_surfaces[1], (x_offset + 20 + plot_width//2 + 10, 20 + plot_height + 10))
        
        # Row 3
        self.screen.blit(error_surface, (x_offset + 20, 20 + 2*(plot_height + 10)))
        
        # Row 4
        self.screen.blit(torque_surface, (x_offset + 20, 20 + 3*(plot_height + 10)))
    
    def draw_hud(self, t: float, kp: float, kd: float, 
                error: np.ndarray, settled: bool,
                controller_on: bool, grav_grad_on: bool, 
                noise_on: bool, mc_status: str, fps: float):
        """Draw HUD overlay."""
        # Background
        hud_rect = pygame.Rect(0, 0, self.config.screen_width, 40)
        pygame.draw.rect(self.screen, (50, 50, 50), hud_rect)
        
        # Text
        text_lines = [
            f"t = {t:.2f}s  |  kp = {kp:.2f}  kd = {kd:.2f}",
            f"||q_err|| = {np.linalg.norm(error):.6f}  |  settled: {settled}",
            f"Controller: {'ON' if controller_on else 'OFF'}  |  "
            f"GravGrad: {'ON' if grav_grad_on else 'OFF'}  |  "
            f"Noise: {'ON' if noise_on else 'OFF'}",
            f"MC status: {mc_status}  |  FPS: {fps:.1f}"
        ]
        
        y = 5
        for line in text_lines:
            text = self.font_small.render(line, True, (200, 200, 200))
            self.screen.blit(text, (10, y))
            y += 22
    
    def draw(self, t: float, q: np.ndarray, omega: np.ndarray,
            error: np.ndarray, torque: np.ndarray,
            kp: float, kd: float, settled: bool,
            controller_on: bool, grav_grad_on: bool,
            noise_on: bool, mc_status: str, fps: float):
        """Draw complete visualization."""
        # Clear screen
        self.screen.fill((0, 0, 0))
        
        # Draw 3D spacecraft
        self.draw_spacecraft(q)
        
        # Draw telemetry
        self.draw_telemetry()
        
        # Draw HUD
        self.draw_hud(t, kp, kd, error, settled, controller_on,
                     grav_grad_on, noise_on, mc_status, fps)
        
        # Update display
        pygame.display.flip()
        self.clock.tick(self.config.fps_target)
        
        return self.clock.get_fps()
    
    def save_plot(self, filename: str):
        """Save current visualization to file."""
        pygame.image.save(self.screen, filename)
    
    def quit(self):
        """Clean up pygame resources."""
        pygame.quit()
