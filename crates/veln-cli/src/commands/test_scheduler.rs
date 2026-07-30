use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

#[derive(Debug)]
pub(crate) enum SchedulerError<E> {
    InvalidBound,
    Job(E),
    WorkerPanicked,
}

pub(crate) fn run_ordered_bounded<J, R, E, F>(
    jobs: Vec<J>,
    bound: usize,
    execute: F,
) -> Result<Vec<R>, SchedulerError<E>>
where
    J: Send,
    R: Send,
    E: Send,
    F: Fn(J) -> Result<R, E> + Sync,
{
    if bound == 0 {
        return Err(SchedulerError::InvalidBound);
    }
    if jobs.is_empty() {
        return Ok(Vec::new());
    }

    let job_count = jobs.len();
    let worker_count = bound.min(job_count);
    let queue = Arc::new(Mutex::new(
        jobs.into_iter().enumerate().collect::<VecDeque<_>>(),
    ));
    let mut records = Vec::with_capacity(job_count);
    records.resize_with(job_count, || None);
    let mut first_error = None;
    let mut worker_panicked = false;

    thread::scope(|scope| {
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let queue = Arc::clone(&queue);
            let tx = tx.clone();
            let execute = &execute;
            handles.push(scope.spawn(move || {
                loop {
                    let Some((index, job)) = queue
                        .lock()
                        .expect("test scheduler queue poisoned")
                        .pop_front()
                    else {
                        break;
                    };
                    if tx.send((index, execute(job))).is_err() {
                        break;
                    }
                }
            }));
        }
        drop(tx);

        for (index, result) in rx {
            match result {
                Ok(record) => records[index] = Some(record),
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }

        for handle in handles {
            if handle.join().is_err() {
                worker_panicked = true;
            }
        }
    });

    if let Some(error) = first_error {
        return Err(SchedulerError::Job(error));
    }
    if worker_panicked {
        return Err(SchedulerError::WorkerPanicked);
    }

    Ok(records
        .into_iter()
        .map(|record| record.expect("finished scheduler job should have a record"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{SchedulerError, run_ordered_bounded};
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Condvar, Mutex};

    #[test]
    fn overlaps_jobs_when_bound_is_greater_than_one() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(0), Condvar::new()));

        let records = run_ordered_bounded(vec![0, 1], 2, {
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            let gate = Arc::clone(&gate);
            move |job| {
                let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(now_active, Ordering::SeqCst);

                let (lock, cvar) = &*gate;
                let mut started = lock.lock().expect("started count should lock");
                *started += 1;
                if *started == 2 {
                    cvar.notify_all();
                }
                while *started < 2 {
                    started = cvar.wait(started).expect("started count should lock");
                }

                active.fetch_sub(1, Ordering::SeqCst);
                Ok::<_, ()>(job)
            }
        })
        .expect("scheduler should complete");

        assert_eq!(records, vec![0, 1]);
        assert_eq!(max_active.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn does_not_overlap_jobs_when_bound_is_one() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));

        let records = run_ordered_bounded(vec![0, 1, 2], 1, {
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            move |job| {
                let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(now_active, Ordering::SeqCst);
                active.fetch_sub(1, Ordering::SeqCst);
                Ok::<_, ()>(job)
            }
        })
        .expect("scheduler should complete");

        assert_eq!(records, vec![0, 1, 2]);
        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn never_exceeds_configured_concurrency_bound() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(BTreeSet::new()), Condvar::new()));

        let records = run_ordered_bounded(vec![0, 1, 2, 3, 4], 3, {
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            let gate = Arc::clone(&gate);
            move |job| {
                let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(now_active, Ordering::SeqCst);

                let (lock, cvar) = &*gate;
                let mut started = lock.lock().expect("started set should lock");
                started.insert(job);
                cvar.notify_all();
                while started.len() < 3 {
                    started = cvar.wait(started).expect("started set should lock");
                }
                drop(started);

                active.fetch_sub(1, Ordering::SeqCst);
                Ok::<_, ()>(job)
            }
        })
        .expect("scheduler should complete");

        assert_eq!(records, vec![0, 1, 2, 3, 4]);
        assert_eq!(max_active.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn returns_records_in_input_order_after_reverse_completion() {
        let gate = Arc::new((
            Mutex::new(ReverseGate {
                started: BTreeSet::new(),
                next_to_finish: 2,
            }),
            Condvar::new(),
        ));

        let records = run_ordered_bounded(vec![0, 1, 2], 3, {
            let gate = Arc::clone(&gate);
            move |job| {
                let (lock, cvar) = &*gate;
                let mut state = lock.lock().expect("reverse gate should lock");
                state.started.insert(job);
                cvar.notify_all();
                while state.started.len() < 3 {
                    state = cvar.wait(state).expect("reverse gate should lock");
                }
                while state.next_to_finish != job {
                    state = cvar.wait(state).expect("reverse gate should lock");
                }
                state.next_to_finish = state.next_to_finish.saturating_sub(1);
                cvar.notify_all();
                Ok::<_, ()>(job)
            }
        })
        .expect("scheduler should complete");

        assert_eq!(records, vec![0, 1, 2]);
    }

    #[test]
    fn returns_records_in_input_order_after_mixed_completion() {
        let gate = Arc::new((Mutex::new(MixedGate::default()), Condvar::new()));

        let records = run_ordered_bounded(vec![0, 1, 2], 3, {
            let gate = Arc::clone(&gate);
            move |job| {
                let (lock, cvar) = &*gate;
                let mut state = lock.lock().expect("mixed gate should lock");
                state.started.insert(job);
                cvar.notify_all();
                while state.started.len() < 3 {
                    state = cvar.wait(state).expect("mixed gate should lock");
                }
                let finish_slot = [1, 2, 0]
                    .iter()
                    .position(|candidate| *candidate == job)
                    .expect("job should be in completion plan");
                while state.next_finish_slot != finish_slot {
                    state = cvar.wait(state).expect("mixed gate should lock");
                }
                state.next_finish_slot += 1;
                cvar.notify_all();
                Ok::<_, ()>(job)
            }
        })
        .expect("scheduler should complete");

        assert_eq!(records, vec![0, 1, 2]);
    }

    #[test]
    fn normal_job_failure_does_not_cancel_remaining_work() {
        let completed = Arc::new(Mutex::new(Vec::new()));

        let records = run_ordered_bounded(vec![0, 1, 2, 3], 2, {
            let completed = Arc::clone(&completed);
            move |job| {
                completed
                    .lock()
                    .expect("completed jobs should lock")
                    .push(job);
                Ok::<_, ()>((job, job == 1))
            }
        })
        .expect("scheduler should complete");

        assert_eq!(records, vec![(0, false), (1, true), (2, false), (3, false)]);
        assert_eq!(
            completed.lock().expect("completed jobs should lock").len(),
            4
        );
    }

    #[test]
    fn joins_workers_before_returning_orchestration_error() {
        let completed = Arc::new(AtomicUsize::new(0));
        let result = run_ordered_bounded(vec![0, 1, 2, 3], 2, {
            let completed = Arc::clone(&completed);
            move |job| {
                completed.fetch_add(1, Ordering::SeqCst);
                if job == 1 { Err("injected") } else { Ok(job) }
            }
        });

        assert!(matches!(result, Err(SchedulerError::Job("injected"))));
        assert_eq!(completed.load(Ordering::SeqCst), 4);
    }

    struct ReverseGate {
        started: BTreeSet<usize>,
        next_to_finish: usize,
    }

    #[derive(Default)]
    struct MixedGate {
        started: BTreeSet<usize>,
        next_finish_slot: usize,
    }
}
