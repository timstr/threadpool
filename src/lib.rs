#[cfg(test)]
mod test;

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

struct ForeachWorkItem<T, F> {
    array: *mut T,
    next_index: Arc<AtomicUsize>,
    length: usize,
    work_function: F,
    barrier: Arc<Barrier>,
}

unsafe impl<T, F> Sync for ForeachWorkItem<T, F>
where
    T: 'static + Sync + Send,
    F: Sync + Send,
{
}

unsafe impl<T, F> Send for ForeachWorkItem<T, F>
where
    T: 'static + Sync + Send,
    F: Sync + Send,
{
}

impl<T, F: FnMut(&mut T)> AnyWorkItem for ForeachWorkItem<T, F>
where
    ForeachWorkItem<T, F>: Sync + Send,
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
struct MapWorkItem<T, R, F> {
    array_in: *const T,
    array_out: *mut Option<R>,
    next_index: Arc<AtomicUsize>,
    length: usize,
    work_function: F,
    barrier: Arc<Barrier>,
}

unsafe impl<T, R, F> Sync for MapWorkItem<T, R, F>
where
    T: 'static + Sync + Send,
    R: 'static + Sync + Send,
    F: Sync + Send,
{
}

unsafe impl<T, R, F> Send for MapWorkItem<T, R, F>
where
    T: 'static + Sync + Send,
    R: 'static + Sync + Send,
    F: Sync + Send,
{
}

impl<T, R, F: FnMut(&T) -> R> AnyWorkItem for MapWorkItem<T, R, F>
where
    MapWorkItem<T, R, F>: Sync + Send,
{
    fn run(mut self: Box<Self>) {
        loop {
            let index = self.next_index.fetch_add(1, Ordering::Relaxed);
            if index >= self.length {
                break;
            }
            let item_in: &T = unsafe { &*self.array_in.add(index) };
            let item_out: &mut Option<R> = unsafe { &mut *self.array_out.add(index) };
            assert!(item_out.is_none());
            *item_out = Some((self.work_function)(item_in));
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

    pub fn new_max_parallelism() -> ThreadPool {
        ThreadPool::new(std::thread::available_parallelism().unwrap().get())
    }

    pub fn foreach<'a, T, F: FnMut(&mut T)>(&mut self, array: &'a mut [T], f: F)
    where
        F: Clone + Sync + Send,
        T: 'static + Sync + Send,
    {
        let next_index = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(self.threads.len() + 1));
        for (_, sender) in &self.threads {
            let workitem = ForeachWorkItem {
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

    pub fn map<'a, T, R, F: FnMut(&T) -> R>(&mut self, array: &'a [T], f: F) -> Vec<R>
    where
        F: Clone + Sync + Send,
        T: 'static + Sync + Send,
        R: 'static + Sync + Send,
    {
        let next_index = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(self.threads.len() + 1));
        // TODO: find out how to use uninitialized values to avoid using Option and
        // a second Vec allocation here
        let mut out: Vec<Option<R>> = Vec::new();
        out.resize_with(array.len(), || None);
        for (_, sender) in &self.threads {
            let workitem = MapWorkItem {
                array_in: array.as_ptr(),
                array_out: out.as_mut_ptr(),
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
        out.into_iter().map(|x| x.unwrap()).collect()
    }

    unsafe fn make_static<'a>(
        workitem: Box<dyn 'a + AnyWorkItem>,
    ) -> Box<dyn 'static + AnyWorkItem> {
        std::mem::transmute(workitem)
    }
}
