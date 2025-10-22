//! Task scheduling

use crate::TaskExecutor;
use chrono::{DateTime, Utc};
use cron::Schedule as CronParser;
use std::str::FromStr;

/// Cron-like schedule for periodic tasks
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::CronSchedule;
///
/// let schedule = CronSchedule::new("0 0 * * *".to_string());
/// assert_eq!(schedule.expression, "0 0 * * *");
/// ```
#[derive(Debug, Clone)]
pub struct CronSchedule {
    pub expression: String,
}

impl CronSchedule {
    /// Create a new cron schedule
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_tasks::CronSchedule;
    ///
    // Run every day at midnight
    /// let daily = CronSchedule::new("0 0 * * *".to_string());
    ///
    // Run every hour
    /// let hourly = CronSchedule::new("0 * * * *".to_string());
    /// ```
    pub fn new(expression: String) -> Self {
        Self { expression }
    }

    /// Calculate next run time based on cron expression
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_tasks::CronSchedule;
    ///
    /// let schedule = CronSchedule::new("0 0 * * *".to_string());
    // Returns the next midnight UTC
    /// let next = schedule.next_run();
    /// assert!(next.is_some());
    /// ```
    pub fn next_run(&self) -> Option<DateTime<Utc>> {
        // Parse cron expression
        let schedule = CronParser::from_str(&self.expression).ok()?;

        // Calculate next run time
        schedule.upcoming(Utc).next()
    }
}

pub trait Schedule: Send + Sync {
    fn next_run(&self) -> Option<DateTime<Utc>>;
}

impl Schedule for CronSchedule {
    fn next_run(&self) -> Option<DateTime<Utc>> {
        CronSchedule::next_run(self)
    }
}

/// Task scheduler for managing periodic tasks
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::Scheduler;
///
/// let scheduler = Scheduler::new();
// Add tasks and run scheduler
/// ```
pub struct Scheduler {
    tasks: Vec<(Box<dyn TaskExecutor>, Box<dyn Schedule>)>,
}

impl Scheduler {
    /// Create a new scheduler
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_tasks::Scheduler;
    ///
    /// let scheduler = Scheduler::new();
    /// ```
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Add a task with schedule
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use reinhardt_tasks::{Scheduler, CronSchedule};
    ///
    /// let mut scheduler = Scheduler::new();
    /// let schedule = CronSchedule::new("0 0 * * *".to_string());
    // scheduler.add_task(Box::new(my_task), Box::new(schedule));
    /// ```
    pub fn add_task(&mut self, task: Box<dyn TaskExecutor>, schedule: Box<dyn Schedule>) {
        self.tasks.push((task, schedule));
    }

    /// Run the scheduler
    ///
    /// This method continuously runs the scheduler, checking each task's schedule
    /// and executing tasks when their scheduled time arrives.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use reinhardt_tasks::{Scheduler, CronSchedule};
    ///
    /// let mut scheduler = Scheduler::new();
    // Add tasks...
    /// scheduler.run().await;
    /// ```
    pub async fn run(&self) {
        use tokio::time::{sleep, Duration};

        loop {
            let now = Utc::now();
            let mut next_check = None;

            // Check each task's schedule
            for (task, schedule) in &self.tasks {
                if let Some(next_run) = schedule.next_run() {
                    // If it's time to run the task
                    if next_run <= now {
                        // Execute the task
                        if let Err(e) = task.execute().await {
                            eprintln!("Task execution failed: {}", e);
                        }
                    } else {
                        // Track the earliest next run time
                        match next_check {
                            None => next_check = Some(next_run),
                            Some(current) if next_run < current => next_check = Some(next_run),
                            _ => {}
                        }
                    }
                }
            }

            // Sleep until the next scheduled task
            if let Some(next) = next_check {
                let duration = (next - now).to_std().unwrap_or(Duration::from_secs(1));
                sleep(duration).await;
            } else {
                // No tasks scheduled, check again in 60 seconds
                sleep(Duration::from_secs(60)).await;
            }
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
