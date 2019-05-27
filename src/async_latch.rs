use std::sync::mpsc::{channel, Sender};
use std::{sync, thread};

pub fn wait(jobs: Vec<sync::Arc<Fn() -> () + Sync + Send>>) {
    let (tx, rx) = channel();
    let total_jobs = jobs.len();
    for job in jobs {
        // tx (Sender) must be moved so the thread is forced to take ownership;
        // tx must be cloned because Sender does not implement Copy thus calling tx.send in the closure would be a use of moved value tx
        let tx = Sender::clone(&tx);
        thread::spawn(move || {
            job();
            // Only unwrapping to satisfy compiler warning; I have no idea what I would do if tx.send failed
            tx.send(()).unwrap();
        });
    }

    for _ in 0..total_jobs {
        rx.recv().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time;

    #[test]
    fn it_works() {
        let a = sync::Arc::new(|| {
            let wait_time = 100;
            println!("sleeping for {:?}", wait_time);
            thread::sleep(time::Duration::from_millis(wait_time as u64));
            println!("awaking after {:?}", wait_time)
        });

        let b = sync::Arc::new(|| {
            let wait_time = 50;
            println!("sleeping for {:?}", wait_time);
            thread::sleep(time::Duration::from_millis(wait_time as u64));
            println!("awaking after {:?}", wait_time)
        });

        wait(vec![a, b])
    }
}
