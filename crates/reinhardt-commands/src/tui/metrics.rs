use sysinfo::{Pid, ProcessRefreshKind, System};
use tokio::sync::mpsc;

/// Process status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
	Running,
	Crashed,
	NotStarted,
}

/// Server metrics
#[derive(Debug, Clone)]
pub struct ServerMetrics {
	pub backend_status: ProcessStatus,
	pub frontend_status: ProcessStatus,
	pub backend_memory_mb: f64,
	pub frontend_memory_mb: f64,
	pub backend_cpu_percent: f32,
	pub frontend_cpu_percent: f32,
}

impl Default for ServerMetrics {
	fn default() -> Self {
		Self {
			backend_status: ProcessStatus::NotStarted,
			frontend_status: ProcessStatus::NotStarted,
			backend_memory_mb: 0.0,
			frontend_memory_mb: 0.0,
			backend_cpu_percent: 0.0,
			frontend_cpu_percent: 0.0,
		}
	}
}

/// Metrics collector
pub struct MetricsCollector {
	system: System,
	backend_pid: Option<Pid>,
	frontend_pid: Option<Pid>,
}

impl Default for MetricsCollector {
	fn default() -> Self {
		Self::new()
	}
}

impl MetricsCollector {
	/// Create a new metrics collector
	pub fn new() -> Self {
		Self {
			system: System::new_all(),
			backend_pid: None,
			frontend_pid: None,
		}
	}

	/// Set backend process PID
	pub fn set_backend_pid(&mut self, pid: u32) {
		self.backend_pid = Some(Pid::from_u32(pid));
	}

	/// Set frontend process PID
	pub fn set_frontend_pid(&mut self, pid: u32) {
		self.frontend_pid = Some(Pid::from_u32(pid));
	}

	/// Update and collect metrics
	pub fn collect(&mut self) -> ServerMetrics {
		// Refresh process information
		self.system.refresh_processes_specifics(
			sysinfo::ProcessesToUpdate::All,
			true,
			ProcessRefreshKind::new().with_cpu().with_memory(),
		);

		let mut metrics = ServerMetrics::default();

		// Collect backend metrics
		if let Some(pid) = self.backend_pid {
			if let Some(process) = self.system.process(pid) {
				metrics.backend_status = ProcessStatus::Running;
				metrics.backend_memory_mb = process.memory() as f64 / 1024.0 / 1024.0;
				metrics.backend_cpu_percent = process.cpu_usage();
			} else {
				metrics.backend_status = ProcessStatus::Crashed;
			}
		}

		// Collect frontend metrics
		if let Some(pid) = self.frontend_pid {
			if let Some(process) = self.system.process(pid) {
				metrics.frontend_status = ProcessStatus::Running;
				metrics.frontend_memory_mb = process.memory() as f64 / 1024.0 / 1024.0;
				metrics.frontend_cpu_percent = process.cpu_usage();
			} else {
				metrics.frontend_status = ProcessStatus::Crashed;
			}
		}

		metrics
	}
}

/// PID update message
#[derive(Debug, Clone, Copy)]
pub enum PidUpdate {
	BackendPid(u32),
	FrontendPid(u32),
	BackendStatus(ProcessStatus),
	FrontendStatus(ProcessStatus),
}

/// Spawn a task to collect metrics periodically
pub fn spawn_metrics_collector_task(
	mut collector: MetricsCollector,
	metrics_tx: mpsc::UnboundedSender<ServerMetrics>,
	mut pid_rx: mpsc::UnboundedReceiver<PidUpdate>,
) -> tokio::task::JoinHandle<()> {
	tokio::spawn(async move {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
		let mut backend_manual_status: Option<ProcessStatus> = None;
		let mut frontend_manual_status: Option<ProcessStatus> = None;

		loop {
			tokio::select! {
				_ = interval.tick() => {
					let mut metrics = collector.collect();

					// Override with manual status if set
					if let Some(status) = backend_manual_status {
						metrics.backend_status = status;
					}
					if let Some(status) = frontend_manual_status {
						metrics.frontend_status = status;
					}

					if metrics_tx.send(metrics).is_err() {
						// Receiver dropped, exit
						break;
					}
				}
				Some(pid_update) = pid_rx.recv() => {
					match pid_update {
						PidUpdate::BackendPid(pid) => {
							collector.set_backend_pid(pid);
							backend_manual_status = None; // Clear manual status when PID is set
						}
						PidUpdate::FrontendPid(pid) => {
							collector.set_frontend_pid(pid);
							frontend_manual_status = None; // Clear manual status when PID is set
						}
						PidUpdate::BackendStatus(status) => {
							backend_manual_status = Some(status);
						}
						PidUpdate::FrontendStatus(status) => {
							frontend_manual_status = Some(status);
						}
					}
				}
				else => break,
			}
		}
	})
}
