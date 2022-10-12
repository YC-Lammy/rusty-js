use std::collections::HashMap;

pub use corosensei::{stack::DefaultStack, Coroutine, CoroutineResult, Yielder};

pub enum AsyncState<T: 'static> {
    Pending(Coroutine<T, T, Result<T, T>, DefaultStack>),
    Resolved(T),
    Rejected(T),
}

pub enum AsyncResult<T: 'static> {
    Yield(T),
    Return(T),
    Err(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsyncId(u64);

pub struct AsyncExecutor<T: 'static> {
    jobs: HashMap<u64, AsyncState<T>>,
    next_id: u64,
}

thread_local! {
    static YIELDER:*const Yielder<(), ()> = std::ptr::null();
}

impl<T> AsyncExecutor<T> {
    pub fn new() -> Self {
        Self {
            jobs: Default::default(),
            next_id: 0,
        }
    }
}

impl<T: 'static> AsyncExecutor<T> {
    pub fn suspend(&self, yielding: T) -> T {
        let yielder = YIELDER.with(|y| {
            if y.is_null() {
                panic!("yielding on non coroutine")
            };
            unsafe { (*y as *const Yielder<T, T>).as_ref().unwrap() }
        });

        yielder.suspend(yielding)
    }

    pub fn run<F>(&mut self, func: F, start: bool) -> AsyncId
    where
        F: Fn() -> Result<T, T> + 'static,
    {
        let id = self.next_id;
        self.next_id += 1;

        let gn = Coroutine::new(move |yielder: &Yielder<T, T>, _input: T| {
            let yielder_ptr = yielder as *const Yielder<T, T> as *const Yielder<(), ()>;

            let old = YIELDER.with(|y| {
                let old = *y;
                let p = y as *const *const Yielder<(), ()> as *mut *const Yielder<(), ()>;
                unsafe { *p = yielder_ptr };
                old
            });

            let re = func();

            // resume the old yielder
            YIELDER.with(|y| {
                let p = y as *const *const Yielder<(), ()> as *mut *const Yielder<(), ()>;
                unsafe { *p = old };
                old
            });

            return re;
        });

        self.jobs.insert(id, AsyncState::Pending(gn));

        if start {
            // start the coroutine
            self.poll(AsyncId(id), unsafe { std::mem::zeroed() });
        }

        AsyncId(id)
    }

    pub fn poll(&mut self, id: AsyncId, input: T) -> bool {
        if let Some(state) = self.jobs.get_mut(&id.0) {
            match state {
                AsyncState::Pending(p) => {
                    let re = p.resume(input);
                    match re {
                        CoroutineResult::Return(r) => match r {
                            Ok(v) => {
                                self.jobs.insert(id.0, AsyncState::Resolved(v));
                                return true;
                            }
                            Err(e) => {
                                self.jobs.insert(id.0, AsyncState::Rejected(e));
                                return false;
                            }
                        },
                        CoroutineResult::Yield(_) => return false,
                    }
                }
                AsyncState::Resolved(_) => return true,
                AsyncState::Rejected(_) => return false,
            };
        } else {
            true
        }
    }
}

impl<T: 'static + Clone> AsyncExecutor<T> {
    pub fn poll_result(&mut self, id: AsyncId, input: T) -> AsyncResult<T> {
        if let Some(state) = self.jobs.get_mut(&id.0) {
            match state {
                AsyncState::Pending(p) => {
                    let re = p.resume(input);
                    match re {
                        CoroutineResult::Return(r) => match r {
                            Ok(v) => {
                                self.jobs.insert(id.0, AsyncState::Resolved(v.clone()));
                                return AsyncResult::Return(v);
                            }
                            Err(e) => {
                                self.jobs.insert(id.0, AsyncState::Rejected(e.clone()));
                                return AsyncResult::Err(e);
                            }
                        },
                        CoroutineResult::Yield(v) => return AsyncResult::Yield(v),
                    }
                }
                AsyncState::Resolved(r) => return AsyncResult::Return(r.clone()),
                AsyncState::Rejected(r) => return AsyncResult::Err(r.clone()),
            };
        } else {
            panic!("non existing async id")
        }
    }

    pub fn finish_all(&mut self, input: T) {
        let mut finish = true;

        loop {
            for id in 0..self.next_id {
                let f = self.poll(AsyncId(id), input.clone());
                if !f {
                    finish = false;
                }
            }
            if finish {
                break;
            }
        }
    }
}
