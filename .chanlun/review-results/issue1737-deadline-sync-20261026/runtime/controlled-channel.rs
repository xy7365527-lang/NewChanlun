use std::sync::mpsc;
use std::time::{Duration, Instant};
fn main() {
    let mode = std::env::args().nth(1).unwrap();
    let (job_tx, job_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let receiver = std::thread::spawn(move || {
        let (reply_tx, reply_rx) = mpsc::sync_channel::<()>(1);
        job_tx.send((Instant::now() + Duration::from_millis(10), reply_tx)).unwrap();
        release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(reply_rx);
        done_tx.send(()).unwrap();
    });
    let (deadline, reply_tx) = job_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
    assert!(Instant::now() >= deadline);
    if mode == "old" {
        assert!(reply_tx.try_send(()).is_err(), "old assertion failed: deadline expired while receiver is held alive");
    } else {
        assert!(reply_tx.try_send(()).is_ok());
        release_tx.send(()).unwrap();
        done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(reply_tx.try_send(()).is_err());
        receiver.join().unwrap();
        println!("CONTROLLED_SYNC_PASS");
    }
}
