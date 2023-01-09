use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    task::{Poll, Waker},
    time::Duration,
};

mod timewheel;
use timewheel::*;

#[derive(Clone)]
pub struct TimerExecutor {
    tick_duration: Duration,
    inner: Arc<Mutex<TimerExecutorImpl>>,
}

struct TimerExecutorImpl {
    timer_id_seq: usize,
    wheel: TimeWheel<usize>,
    wakers: HashMap<usize, std::task::Waker>,
    fired: HashSet<usize>,
}

impl TimerExecutorImpl {
    fn new(step: u64) -> Self {
        Self {
            timer_id_seq: 0,
            wheel: TimeWheel::new(step),
            wakers: Default::default(),
            fired: Default::default(),
        }
    }

    fn create_timer(&mut self, duration: u64) -> usize {
        self.timer_id_seq += 1;

        let timer = self.timer_id_seq;

        self.wheel.add(duration, timer);

        timer
    }

    fn poll(&mut self, timer: usize, waker: Waker) -> Poll<()> {
        if self.fired.remove(&timer) {
            Poll::Ready(())
        } else {
            log::debug!("inser timer {} waker", timer);
            self.wakers.insert(timer, waker);
            Poll::Pending
        }
    }

    fn tick(&mut self) {
        if let Poll::Ready(timers) = self.wheel.tick() {
            log::debug!("ready timers {:?}", timers);
            for timer in timers {
                self.fired.insert(timer);

                if let Some(waker) = self.wakers.remove(&timer) {
                    log::debug!("wake up timer {}", timer);
                    waker.wake_by_ref();
                }
            }
        }
    }
}

impl TimerExecutor {
    pub fn new(step: u64, tick_duration: Duration) -> Self {
        let inner: Arc<Mutex<TimerExecutorImpl>> =
            Arc::new(Mutex::new(TimerExecutorImpl::new(step)));

        let inner_tick = inner.clone();

        std::thread::spawn(move || {
            // When no other strong reference is alive, stop tick thread
            while Arc::strong_count(&inner_tick) > 1 {
                inner_tick.lock().unwrap().tick();

                std::thread::sleep(tick_duration);
            }
        });

        Self {
            inner,
            tick_duration,
        }
    }

    /// Create a new timeout future instance.
    pub fn timeout(&self, duration: Duration) -> Timeout {
        let mut ticks = duration.as_millis() / self.tick_duration.as_millis();

        if ticks == 0 {
            ticks = 1;
        }

        let timer_id = self.inner.lock().unwrap().create_timer(ticks as u64);

        Timeout {
            timer_id,
            executor: self.inner.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Timeout {
    timer_id: usize,
    executor: Arc<Mutex<TimerExecutorImpl>>,
}

impl std::future::Future for Timeout {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.executor
            .lock()
            .unwrap()
            .poll(self.timer_id, cx.waker().clone())
    }
}

impl crate::Timer for Timeout {
    fn new(duration: Duration) -> Self {
        global_timer_executor().timeout(duration)
    }
}

impl crate::TimerWithContext for Timeout {
    type Context = TimerExecutor;
    fn new_with_context<C>(duration: Duration, mut context: C) -> Self
    where
        C: AsMut<Self::Context>,
    {
        context.as_mut().timeout(duration)
    }
}

/// Accesss global static timer executor instance
pub fn global_timer_executor() -> &'static TimerExecutor {
    use once_cell::sync::OnceCell;

    static INSTANCE: OnceCell<TimerExecutor> = OnceCell::new();

    INSTANCE.get_or_init(|| TimerExecutor::new(3600, Duration::from_millis(10)))
}
