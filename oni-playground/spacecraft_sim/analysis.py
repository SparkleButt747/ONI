"""
Analysis module: Post-run matplotlib plots.

Produces two figures:
1. Nominal run (2×2 subplot):
   - [0,0]: quaternion vs time
   - [0,1]: ω vs time
   - [1,0]: ||q_err|| log scale vs time, mark settling time
   - [1,1]: torque magnitude vs time, hline at τ_max

2. MC results (2×2 subplot):
   - [0,0]: histogram of settling times (20 bins, mark mean)
   - [0,1]: scatter: kp vs settling_time, colour=kd, cmap=viridis
   - [1,0]: scatter: kd vs ss_error (log y)
   - [1,1]: CDF of settling times
"""

import numpy as np
import matplotlib.pyplot as plt
from matplotlib.gridspec import GridSpec
from typing import Tuple, Optional, List
import os


def plot_nominal_run(times: np.ndarray, states: np.ndarray, 
                    torques: np.ndarray, errors: np.ndarray,
                    settling_time: float, tau_max: float = 5.0,
                    save_path: str = "./output/nominal.png"):
    """
    Plot nominal run results.
    
    2×2 subplot:
    [0,0]: quaternion vs time
    [0,1]: ω vs time
    [1,0]: ||q_err|| log scale vs time, mark settling time
    [1,1]: torque magnitude vs time, hline at τ_max
    """
    fig = plt.figure(figsize=(14, 10))
    gs = GridSpec(2, 2, figure=fig)
    
    # Extract data
    q = states[:, 0:4]
    omega = states[:, 4:7]
    torque_mag = np.linalg.norm(torques, axis=1)
    error_mag = np.linalg.norm(errors, axis=1)
    
    # Plot 1: Quaternions
    ax00 = fig.add_subplot(gs[0, 0])
    colors = ['k', 'r', 'g', 'b']
    labels = ['q0', 'q1', 'q2', 'q3']
    for i in range(4):
        ax00.plot(times, q[:, i], color=colors[i], label=labels[i], linewidth=1.5)
    ax00.set_xlabel('Time (s)')
    ax00.set_ylabel('Quaternion Component')
    ax00.set_title('Quaternion Components vs Time')
    ax00.legend(loc='upper right')
    ax00.grid(True, alpha=0.3)
    ax00.set_ylim([-1.1, 1.1])
    
    # Plot 2: Angular velocity
    ax01 = fig.add_subplot(gs[0, 1])
    colors = ['r', 'g', 'b']
    labels = ['ω_x', 'ω_y', 'ω_z']
    for i in range(3):
        ax01.plot(times, omega[:, i], color=colors[i], label=labels[i], linewidth=1.5)
    ax01.set_xlabel('Time (s)')
    ax01.set_ylabel('Angular Velocity (rad/s)')
    ax01.set_title('Angular Velocity vs Time')
    ax01.legend(loc='upper right')
    ax01.grid(True, alpha=0.3)
    
    # Plot 3: Error magnitude (log scale)
    ax10 = fig.add_subplot(gs[1, 0])
    ax10.semilogy(times, error_mag, 'b', linewidth=1.5)
    ax10.set_xlabel('Time (s)')
    ax10.set_ylabel('||q_err|| (log scale)')
    ax10.set_title('Attitude Error Magnitude vs Time')
    ax10.grid(True, alpha=0.3, which='both')
    
    # Mark settling time
    if not np.isnan(settling_time):
        ax10.axvline(x=settling_time, color='r', linestyle='--', 
                    label=f'Settling time = {settling_time:.2f}s')
        ax10.legend()
    
    # Plot 4: Torque magnitude
    ax11 = fig.add_subplot(gs[1, 1])
    ax11.plot(times, torque_mag, 'r', linewidth=1.5)
    ax11.axhline(y=tau_max, color='k', linestyle='--', label=f'τ_max = {tau_max} N·m')
    ax11.set_xlabel('Time (s)')
    ax11.set_ylabel('Torque Magnitude (N·m)')
    ax11.set_title('Control Torque Magnitude vs Time')
    ax11.legend(loc='upper right')
    ax11.grid(True, alpha=0.3)
    
    plt.tight_layout()
    
    # Save
    os.makedirs(os.path.dirname(save_path), exist_ok=True)
    plt.savefig(save_path, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"Nominal run plot saved to: {save_path}")


def plot_monte_carlo(results: List, save_path: str = "./output/montecarlo.png"):
    """
    Plot Monte Carlo results.
    
    2×2 subplot:
    [0,0]: histogram of settling times (20 bins, mark mean)
    [0,1]: scatter: kp vs settling_time, colour=kd, cmap=viridis
    [1,0]: scatter: kd vs ss_error (log y)
    [1,1]: CDF of settling times
    """
    # Extract data
    settling_times = np.array([r.settling_time for r in results if not np.isnan(r.settling_time)])
    ss_errors = np.array([r.ss_error for r in results])
    max_torques = np.array([r.max_torque for r in results])
    kps = np.array([r.kp for r in results])
    kds = np.array([r.kd for r in results])
    
    fig = plt.figure(figsize=(14, 10))
    gs = GridSpec(2, 2, figure=fig)
    
    # Plot 1: Histogram of settling times
    ax00 = fig.add_subplot(gs[0, 0])
    if len(settling_times) > 0:
        n, bins, patches = ax00.hist(settling_times, bins=20, alpha=0.7, 
                                    color='steelblue', edgecolor='black')
        mean_st = np.mean(settling_times)
        ax00.axvline(mean_st, color='red', linestyle='--', linewidth=2,
                    label=f'Mean = {mean_st:.2f}s')
        ax00.set_xlabel('Settling Time (s)')
        ax00.set_ylabel('Count')
        ax00.set_title('Settling Time Distribution')
        ax00.legend()
        ax00.grid(True, alpha=0.3)
    else:
        ax00.text(0.5, 0.5, 'No converged trials', ha='center', va='center',
                 transform=ax00.transAxes, fontsize=12)
        ax00.set_title('Settling Time Distribution (Empty)')
    
    # Plot 2: kp vs settling_time
    ax01 = fig.add_subplot(gs[0, 1])
    if len(settling_times) > 0:
        scatter = ax01.scatter(kps, settling_times, c=kds, cmap='viridis', 
                              alpha=0.7, s=30)
        ax01.set_xlabel('kp')
        ax01.set_ylabel('Settling Time (s)')
        ax01.set_title('kp vs Settling Time (color = kd)')
        plt.colorbar(scatter, ax=ax01, label='kd')
        ax01.grid(True, alpha=0.3)
    else:
        ax01.text(0.5, 0.5, 'No converged trials', ha='center', va='center',
                 transform=ax01.transAxes, fontsize=12)
        ax01.set_title('kp vs Settling Time (Empty)')
    
    # Plot 3: kd vs ss_error (log y)
    ax10 = fig.add_subplot(gs[1, 0])
    scatter = ax10.scatter(kds, ss_errors, c=kps, cmap='plasma', 
                          alpha=0.7, s=30)
    ax10.set_yscale('log')
    ax10.set_xlabel('kd')
    ax10.set_ylabel('Steady-State Error (log scale)')
    ax10.set_title('kd vs Steady-State Error (color = kp)')
    plt.colorbar(scatter, ax=ax10, label='kp')
    ax10.grid(True, alpha=0.3, which='both')
    
    # Plot 4: CDF of settling times
    ax11 = fig.add_subplot(gs[1, 1])
    if len(settling_times) > 0:
        sorted_times = np.sort(settling_times)
        cdf = np.arange(1, len(sorted_times) + 1) / len(sorted_times)
        ax11.plot(sorted_times, cdf, 'b-', linewidth=2)
        ax11.set_xlabel('Settling Time (s)')
        ax11.set_ylabel('Cumulative Probability')
        ax11.set_title('CDF of Settling Times')
        ax11.grid(True, alpha=0.3)
    else:
        ax11.text(0.5, 0.5, 'No converged trials', ha='center', va='center',
                 transform=ax11.transAxes, fontsize=12)
        ax11.set_title('CDF of Settling Times (Empty)')
    
    plt.tight_layout()
    
    # Save
    os.makedirs(os.path.dirname(save_path), exist_ok=True)
    plt.savefig(save_path, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"Monte Carlo plot saved to: {save_path}")


def analyze_run(times: np.ndarray, states: np.ndarray, 
               torques: np.ndarray, errors: np.ndarray,
               settling_time: float, tau_max: float = 5.0):
    """
    Analyze a single run and generate plots.
    
    Returns analysis dictionary.
    """
    # Compute metrics
    error_mag = np.linalg.norm(errors, axis=1)
    torque_mag = np.linalg.norm(torques, axis=1)
    
    # Final error
    final_error = error_mag[-1]
    final_torque = torque_mag[-1]
    
    # Statistics
    error_stats = {
        'mean': np.mean(error_mag),
        'max': np.max(error_mag),
        'final': final_error
    }
    
    torque_stats = {
        'mean': np.mean(torque_mag),
        'max': np.max(torque_mag),
        'final': final_torque
    }
    
    # Print summary
    print("\n" + "=" * 50)
    print("NOMINAL RUN ANALYSIS")
    print("=" * 50)
    print(f"Final attitude error: {final_error:.6e}")
    print(f"Final control torque: {final_torque:.4f} N·m")
    print(f"Settling time: {settling_time:.2f} s")
    print(f"\nError statistics:")
    print(f"  Mean: {error_stats['mean']:.6e}")
    print(f"  Max: {error_stats['max']:.6e}")
    print(f"\nTorque statistics:")
    print(f"  Mean: {torque_stats['mean']:.4f} N·m")
    print(f"  Max: {torque_stats['max']:.4f} N·m")
    print("=" * 50)
    
    # Generate plots
    plot_nominal_run(times, states, torques, errors, settling_time, tau_max)
    
    return {
        'error_stats': error_stats,
        'torque_stats': torque_stats,
        'settling_time': settling_time
    }


def analyze_monte_carlo(results):
    """
    Analyze Monte Carlo results and generate plots.
    
    Returns summary statistics.
    """
    if not results:
        print("No Monte Carlo results to analyze.")
        return None
    
    # Extract data
    converged = [r for r in results if r.converged]
    non_converged = [r for r in results if not r.converged]
    
    conv_rate = 100 * len(converged) / len(results)
    
    settling_times = np.array([r.settling_time for r in converged])
    ss_errors = np.array([r.ss_error for r in results])
    max_torques = np.array([r.max_torque for r in results])
    
    # Statistics
    if len(settling_times) > 0:
        st_mean = np.mean(settling_times)
        st_std = np.std(settling_times)
        st_5th = np.percentile(settling_times, 5)
        st_95th = np.percentile(settling_times, 95)
    else:
        st_mean = st_std = st_5th = st_95th = np.nan
    
    ss_mean = np.mean(ss_errors)
    ss_std = np.std(ss_errors)
    mt_mean = np.mean(max_torques)
    
    # Print summary
    print("\n" + "=" * 50)
    print("MONTE CARLO ANALYSIS")
    print("=" * 50)
    print(f"Total trials: {len(results)}")
    print(f"Converged: {len(converged)} ({conv_rate:.1f}%)")
    print(f"Non-converged: {len(non_converged)}")
    print(f"\nSettling time (converged only):")
    print(f"  Mean ± std: {st_mean:.2f}s ± {st_std:.2f}s")
    print(f"  5th / 95th percentile: {st_5th:.2f}s / {st_95th:.2f}s")
    print(f"\nSteady-state error:")
    print(f"  Mean ± std: {ss_mean:.6e} ± {ss_std:.6e}")
    print(f"\nMax torque:")
    print(f"  Mean: {mt_mean:.4f} N·m")
    print("=" * 50)
    
    # Generate plots
    plot_monte_carlo(results)
    
    return {
        'conv_rate': conv_rate,
        'settling_time_stats': {
            'mean': st_mean,
            'std': st_std,
            'p5': st_5th,
            'p95': st_95th
        },
        'ss_error_stats': {
            'mean': ss_mean,
            'std': ss_std
        },
        'max_torque_stats': {
            'mean': mt_mean
        }
    }
