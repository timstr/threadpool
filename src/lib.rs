use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::Sender,
        Arc, Barrier,
    },
    thread::JoinHandle,
};

trait AnyWorkItem: Sync + Send {
    fn run(self: Box<Self>);
}

struct WorkItem<T, F> {
    array: *mut T,
    next_index: Arc<AtomicUsize>,
    length: usize,
    work_function: F,
    barrier: Arc<Barrier>,
}

unsafe impl<T, F> Sync for WorkItem<T, F>
where
    T: 'static + Sync + Send,
    F: Sync + Send,
{
}

unsafe impl<T, F> Send for WorkItem<T, F>
where
    T: 'static + Sync + Send,
    F: Sync + Send,
{
}

impl<T, F: FnMut(&mut T)> AnyWorkItem for WorkItem<T, F>
where
    WorkItem<T, F>: Sync + Send,
{
    fn run(mut self: Box<Self>) {
        loop {
            let index = self.next_index.fetch_add(1, Ordering::Relaxed);
            if index >= self.length {
                break;
            }
            let item: &mut T = unsafe { &mut *self.array.add(index) };
            (self.work_function)(item);
        }
        self.barrier.wait();
    }
}

enum ThreadPoolMessage {
    DoWork(Box<dyn AnyWorkItem>),
}

pub struct ThreadPool {
    threads: Vec<(JoinHandle<()>, Sender<ThreadPoolMessage>)>,
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> ThreadPool {
        ThreadPool {
            threads: (0..num_threads)
                .map(|_| {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    let jh = std::thread::spawn(move || {
                        while let Ok(msg) = receiver.recv() {
                            match msg {
                                ThreadPoolMessage::DoWork(workitem) => workitem.run(),
                            }
                        }
                    });
                    (jh, sender)
                })
                .collect(),
        }
    }

    pub fn foreach<'a, T, F: FnMut(&mut T)>(&mut self, array: &'a mut [T], f: F)
    where
        F: Clone + Sync + Send,
        T: 'static + Sync + Send,
    {
        let next_index = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(self.threads.len() + 1));
        for (_, sender) in &self.threads {
            let workitem = WorkItem {
                array: array.as_mut_ptr(),
                next_index: Arc::clone(&next_index),
                length: array.len(),
                work_function: f.clone(),
                barrier: Arc::clone(&barrier),
            };
            let workitem: Box<dyn AnyWorkItem> = Box::new(workitem);
            let workitem = unsafe { Self::make_static(workitem) };
            sender.send(ThreadPoolMessage::DoWork(workitem)).unwrap();
        }
        barrier.wait();
    }

    unsafe fn make_static<'a>(
        workitem: Box<dyn 'a + AnyWorkItem>,
    ) -> Box<dyn 'static + AnyWorkItem> {
        std::mem::transmute(workitem)
    }
}
