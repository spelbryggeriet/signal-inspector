#[cfg(debug_assertions)]
thread_local! {
    pub static TIMESTAMPS: std::sync::Mutex<Vec<(f64, String, bool)>> = std::sync::Mutex::new(Vec::with_capacity(16));
}

#[cfg(debug_assertions)]
pub fn now() -> f64 {
    thread_local! {
        static PERFORMANCE: web_sys::Performance = {
            web_sys::window()
                .expect("should have a window in this context")
                .performance()
                .expect("performance should be available")
        }
    }

    PERFORMANCE.with(|p| p.now())
}

#[cfg(debug_assertions)]
macro_rules! bench {
    ([$($fmt:tt)*] => $e:expr) => {{
        bench_start!($($fmt)*);
        let e = $e;
        bench_end!();
        e
    }};
}

#[cfg(debug_assertions)]
macro_rules! bench_start {
    ($($fmt:tt)*) => {{
        let label = format!($($fmt)*);
        $crate::bench::TIMESTAMPS.with(|t| {
            let mut lock = t.lock().expect("should have exclusive access");
            let start = $crate::bench::now();
            lock.iter_mut().for_each(|(_, label, is_non_nested)| {
                if *is_non_nested {
                    $crate::log(&format!("[START] {label}"));
                    *is_non_nested = false;
                }
            });
            lock.push((start, label, true));
        })
    }};
}

#[cfg(debug_assertions)]
macro_rules! bench_end {
    () => {{
        let end = $crate::bench::now();
        $crate::bench::TIMESTAMPS.with(|t| {
            let mut lock = t.lock().expect("should have exclusive access");
            let (start, label, is_non_nested) = lock.pop().expect("should be non-empty");
            let duration = end - start;

            let duration_fmt = if duration >= 1000.0 {
                format!("{:.3} s", duration / 1000.0)
            } else {
                format!("{duration:.1} ms")
            };

            if is_non_nested {
                $crate::log(&format!("{label} = {duration_fmt}",));
            } else {
                $crate::log(&format!("[ END ] {label} = {duration_fmt}"));
            }
        });
    }};
}

#[cfg(not(debug_assertions))]
macro_rules! bench {
    ([$($t:tt)*] => $e:expr) => {
        $e
    };
}

#[cfg(not(debug_assertions))]
macro_rules! bench_start {
    ($($t:tt)*) => {};
}

#[cfg(not(debug_assertions))]
macro_rules! bench_end {
    ($($t:tt)*) => {};
}
