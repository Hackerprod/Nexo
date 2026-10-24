//! Deterministic allocation traffic for isolated compiler scale fixtures.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

thread_local! {
    static CAPTURE: Cell<u8> = const { Cell::new(0) };
    static MEASUREMENT: RefCell<Option<MutexGuard<'static, ()>>> = const { RefCell::new(None) };
}
static MEASUREMENT_LOCK: Mutex<()> = Mutex::new(());
static CAMPAIGN_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn campaign_guard() -> MutexGuard<'static, ()> {
    CAMPAIGN_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
static PARSER_BYTES: AtomicU64 = AtomicU64::new(0);
static PARSER_ALLOCS: AtomicU64 = AtomicU64::new(0);
static DEAD_BYTES: AtomicU64 = AtomicU64::new(0);
static DEAD_ALLOCS: AtomicU64 = AtomicU64::new(0);
static SEMANTIC_BYTES: AtomicU64 = AtomicU64::new(0);
static SEMANTIC_ALLOCS: AtomicU64 = AtomicU64::new(0);
static CFG_BYTES: AtomicU64 = AtomicU64::new(0);
static CFG_ALLOCS: AtomicU64 = AtomicU64::new(0);
static LATER_BYTES: AtomicU64 = AtomicU64::new(0);
static LATER_ALLOCS: AtomicU64 = AtomicU64::new(0);
static PRINT_BYTES: AtomicU64 = AtomicU64::new(0);
static PRINT_ALLOCS: AtomicU64 = AtomicU64::new(0);
static FORMAT_BYTES: AtomicU64 = AtomicU64::new(0);
static FORMAT_ALLOCS: AtomicU64 = AtomicU64::new(0);

struct Counting;
#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn count(bytes: usize) {
    let id = CAPTURE.try_with(Cell::get).unwrap_or(0);
    let (total, count) = match id {
        1 => (&PARSER_BYTES, &PARSER_ALLOCS),
        2 => (&DEAD_BYTES, &DEAD_ALLOCS),
        3 => (&SEMANTIC_BYTES, &SEMANTIC_ALLOCS),
        4 => (&CFG_BYTES, &CFG_ALLOCS),
        5 => (&LATER_BYTES, &LATER_ALLOCS),
        6 => (&PRINT_BYTES, &PRINT_ALLOCS),
        7 => (&FORMAT_BYTES, &FORMAT_ALLOCS),
        _ => return,
    };
    total.fetch_add(bytes as u64, Ordering::Relaxed);
    count.fetch_add(1, Ordering::Relaxed);
}

// SAFETY: all allocation operations delegate to System with the same layout.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            count(layout.size());
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            count(layout.size());
        }
        ptr
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, layout, size) };
        if !result.is_null() {
            count(size);
        }
        result
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            System.dealloc(ptr, layout);
        }
    }
}

pub(crate) struct Guard(u8);
pub(crate) fn capture_for(file: &str) -> Guard {
    let id = match file {
        "parser_scale_counter.nx" => 1,
        "dead_scale_counter.nx" => 2,
        "semantic_scale_counter.nx" => 3,
        "cfg_scale_counter.nx" => 4,
        "later_operand_scale_counter.nx" => 5,
        "print_scale_counter.nx" => 6,
        "format_scale_counter.nx" => 7,
        _ => 0,
    };
    let old = CAPTURE.with(|cell| cell.replace(id));
    Guard(old)
}
impl Drop for Guard {
    fn drop(&mut self) {
        CAPTURE.with(|cell| cell.set(self.0));
    }
}
pub(crate) fn reset(file: &str) {
    MEASUREMENT.with(|slot| {
        let mut held = slot.borrow_mut();
        assert!(held.is_none(), "nested allocation measurement");
        *held = Some(
            MEASUREMENT_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
    });
    let (bytes, count) = match file {
        "parser_scale_counter.nx" => (&PARSER_BYTES, &PARSER_ALLOCS),
        "dead_scale_counter.nx" => (&DEAD_BYTES, &DEAD_ALLOCS),
        "semantic_scale_counter.nx" => (&SEMANTIC_BYTES, &SEMANTIC_ALLOCS),
        "cfg_scale_counter.nx" => (&CFG_BYTES, &CFG_ALLOCS),
        "later_operand_scale_counter.nx" => (&LATER_BYTES, &LATER_ALLOCS),
        "print_scale_counter.nx" => (&PRINT_BYTES, &PRINT_ALLOCS),
        "format_scale_counter.nx" => (&FORMAT_BYTES, &FORMAT_ALLOCS),
        _ => return,
    };
    bytes.store(0, Ordering::Relaxed);
    count.store(0, Ordering::Relaxed);
}
pub(crate) fn snapshot(file: &str) -> (u64, u64) {
    let (bytes, count) = match file {
        "parser_scale_counter.nx" => (&PARSER_BYTES, &PARSER_ALLOCS),
        "dead_scale_counter.nx" => (&DEAD_BYTES, &DEAD_ALLOCS),
        "semantic_scale_counter.nx" => (&SEMANTIC_BYTES, &SEMANTIC_ALLOCS),
        "cfg_scale_counter.nx" => (&CFG_BYTES, &CFG_ALLOCS),
        "later_operand_scale_counter.nx" => (&LATER_BYTES, &LATER_ALLOCS),
        "print_scale_counter.nx" => (&PRINT_BYTES, &PRINT_ALLOCS),
        "format_scale_counter.nx" => (&FORMAT_BYTES, &FORMAT_ALLOCS),
        _ => return (0, 0),
    };
    let result = (bytes.load(Ordering::Relaxed), count.load(Ordering::Relaxed));
    MEASUREMENT.with(|slot| {
        assert!(slot.borrow_mut().take().is_some(), "snapshot without reset");
    });
    result
}
