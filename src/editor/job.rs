//! Background work. A job is a thread that owns nothing but what it
//! was handed and reports back through the one channel the main loop waits
//! on. The editor is never touched from a worker: a report is data, and what
//! to do with it is decided on arrival.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, mpsc};
use std::thread;

use super::input::Input;
use super::panel::picker::Candidate;
use super::workspace::BufferId;
use crate::buffer::syntax::Fresh;

/// What the main loop wakes up for: a keystroke or the like from the
/// terminal, something a worker has to say, a line from the socket, or
/// the system asking the process to end.
#[derive(Debug)]
pub enum Message {
    Input(Input),
    Report(Report),
    Remote(Remote),
    /// A signal that asks the process to end: SIGTERM, SIGHUP, SIGINT.
    /// Not a command — nobody at the keyboard asked for it, and it is not
    /// for the palette — and not a report, the editor did no work for it.
    Terminate,
}

/// A line from the socket, and where its answer goes. The line is
/// read by the loop, not here: the editor's layer knows nothing of the
/// language it is written in.
#[derive(Debug)]
pub struct Remote {
    pub line: String,
    pub reply: Sender<String>,
}

/// The editor's own work coming back to it. Not a command: nobody asks for
/// a report, it is not nameable, and it may be stale by the time it arrives.
#[derive(Debug)]
pub enum Report {
    /// A batch of a picker's candidates.
    Candidates { job: JobId, batch: Vec<Candidate> },
    /// The syntax tree of a buffer's text, parsed again after an edit. The
    /// buffer checks for itself whether it still wants this tree.
    Parsed { buffer: BufferId, fresh: Fresh },
    /// The file behind a buffer is not what it was; the editor checks for
    /// itself whether that matters.
    FileChanged(BufferId),
    /// The worker is gone, whether it finished or died. Sent by a guard, so
    /// it comes even after a panic.
    Ended(JobId),
}

/// `Default` is for tests that need a job that was never spawned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct JobId(u64);

/// What a worker gets to work with: a way to report, and a flag to check
/// between units of work.
pub struct Worker {
    id: JobId,
    tx: Sender<Message>,
    cancelled: Arc<AtomicBool>,
}

impl Worker {
    pub fn id(&self) -> JobId {
        self.id
    }

    /// False once nobody is listening, which is a reason to stop as good as
    /// being cancelled.
    pub fn report(&self, report: Report) -> bool {
        self.tx.send(Message::Report(report)).is_ok()
    }

    pub fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::Report(Report::Ended(self.id)));
    }
}

struct Live {
    cancelled: Arc<AtomicBool>,
    /// A watcher rather than a task: it ends only when cancelled, so nobody
    /// waits for it to finish.
    perpetual: bool,
}

/// The jobs in flight and the channel they all report into. Dropping this
/// cancels every job; the threads notice at their next check.
pub struct Jobs {
    tx: Sender<Message>,
    next: u64,
    live: HashMap<JobId, Live>,
}

impl Jobs {
    pub fn new() -> (Self, Receiver<Message>) {
        let (tx, rx) = mpsc::channel();
        let jobs = Self {
            tx,
            next: 0,
            live: HashMap::new(),
        };
        (jobs, rx)
    }

    /// Another way in for whoever else has something to say — the thread
    /// reading the terminal, for one.
    pub fn sender(&self) -> Sender<Message> {
        self.tx.clone()
    }

    /// A task: it works, reports, and ends.
    pub fn spawn(&mut self, work: impl FnOnce(Worker) + Send + 'static) -> JobId {
        self.start(work, false)
    }

    /// A watcher: it reports whenever it has something to say, and ends only
    /// when cancelled.
    pub fn spawn_forever(&mut self, work: impl FnOnce(Worker) + Send + 'static) -> JobId {
        self.start(work, true)
    }

    fn start(&mut self, work: impl FnOnce(Worker) + Send + 'static, perpetual: bool) -> JobId {
        let id = JobId(self.next);
        self.next += 1;
        let cancelled = Arc::new(AtomicBool::new(false));
        self.live.insert(
            id,
            Live {
                cancelled: cancelled.clone(),
                perpetual,
            },
        );

        let worker = Worker {
            id,
            tx: self.tx.clone(),
            cancelled,
        };
        thread::spawn(move || work(worker));
        id
    }

    pub fn cancel(&mut self, id: JobId) {
        if let Some(live) = self.live.remove(&id) {
            live.cancelled.store(true, Ordering::Relaxed);
        }
    }

    /// Whether a report from this job is still wanted.
    pub fn is_live(&self, id: JobId) -> bool {
        self.live.contains_key(&id)
    }

    pub fn ended(&mut self, id: JobId) {
        self.live.remove(&id);
    }

    /// Whether any task is still to finish; watchers do not count.
    pub fn any_pending(&self) -> bool {
        self.live.values().any(|live| !live.perpetual)
    }
}

impl Drop for Jobs {
    fn drop(&mut self) {
        for live in self.live.values() {
            live.cancelled.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn next_report(rx: &Receiver<Message>) -> Report {
        match rx.recv().unwrap() {
            Message::Report(report) => report,
            Message::Input(_) | Message::Remote(_) | Message::Terminate => {
                panic!("no input was sent")
            }
        }
    }

    #[test]
    fn a_worker_reports_and_then_says_it_has_ended() {
        let (mut jobs, rx) = Jobs::new();
        let id = jobs.spawn(|worker| {
            worker.report(Report::Candidates {
                job: worker.id(),
                batch: vec![Candidate::plain("one")],
            });
        });

        assert!(matches!(
            next_report(&rx),
            Report::Candidates { job, batch }
                if job == id && batch == [Candidate::plain("one")]
        ));
        assert!(matches!(next_report(&rx), Report::Ended(job) if job == id));
    }

    #[test]
    fn a_worker_that_panics_still_ends() {
        let (mut jobs, rx) = Jobs::new();
        let id = jobs.spawn(|_| panic!("worker down"));
        assert!(matches!(next_report(&rx), Report::Ended(job) if job == id));
    }

    #[test]
    fn a_cancelled_job_is_no_longer_live_and_its_worker_sees_the_flag() {
        let (mut jobs, rx) = Jobs::new();
        let (started, wait) = mpsc::channel();
        let id = jobs.spawn(move |worker| {
            started.send(()).unwrap();
            while !worker.cancelled() {
                thread::yield_now();
            }
        });
        wait.recv().unwrap();
        assert!(jobs.is_live(id));

        jobs.cancel(id);
        assert!(!jobs.is_live(id));
        assert!(matches!(next_report(&rx), Report::Ended(job) if job == id));
    }
}
