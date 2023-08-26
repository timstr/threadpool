use crate::ThreadPool;

#[test]
fn test_threadpool_one_thread() {
    let mut data: Vec<usize> = (0..8192).collect();

    let mut threadpool = ThreadPool::new(1);

    threadpool.foreach(&mut data, |x| *x = *x + *x);

    assert!(data.iter().enumerate().all(|(i, x)| *x == 2 * i));
}

#[test]
fn test_threadpool_two_threads() {
    let mut data: Vec<usize> = (0..8192).collect();

    let mut threadpool = ThreadPool::new(2);

    threadpool.foreach(&mut data, |x| *x = *x + *x);

    assert!(data.iter().enumerate().all(|(i, x)| *x == 2 * i));
}

#[test]
fn test_threadpool_all_threads() {
    let mut data: Vec<usize> = (0..65536).collect();

    let num_threads = std::thread::available_parallelism().unwrap();

    let mut threadpool = ThreadPool::new(num_threads.get());

    threadpool.foreach(&mut data, |x| *x = *x + *x);

    assert!(data.iter().enumerate().all(|(i, x)| *x == 2 * i));
}

#[test]
fn test_threadpool_borrowing() {
    let mut data: Vec<usize> = (0..65536).collect();

    let mut threadpool = ThreadPool::new(1);

    let offset: usize = 10;

    threadpool.foreach(&mut data, |x| *x = *x + offset);

    assert!(data.iter().enumerate().all(|(i, x)| *x == i + 10));
}
