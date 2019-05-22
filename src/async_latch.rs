use std::sync::mpsc::channel;
use std::{sync, thread};

pub fn wait(jobs: Vec<sync::Arc<sync::Mutex<Fn() + Send>>>) {
    let (tx, rx) = channel();
    let total_jobs = jobs.len();
    for job in jobs {
        let tx = tx.clone();
        let j = job.clone();
        thread::spawn(move || {
            j.lock().unwrap()();
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
        let a = sync::Arc::new(sync::Mutex::new(move || {
            let wait_time = 100;
            println!("sleeping for {:?}", wait_time);
            thread::sleep(time::Duration::from_millis(wait_time as u64));
            println!("awaking after {:?}", wait_time)
        }));

        let b = sync::Arc::new(sync::Mutex::new(move || {
            let wait_time = 50;
            println!("sleeping for {:?}", wait_time);
            thread::sleep(time::Duration::from_millis(wait_time as u64));
            println!("awaking after {:?}", wait_time)
        }));

        wait(vec![a, b])
    }
}
