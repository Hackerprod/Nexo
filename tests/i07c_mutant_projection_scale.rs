use nexo::check::check_proposed;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

struct CountingAllocator;

static ALLOCATED: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn source(families: usize, resolved_projection: bool) -> String {
    let mut source = String::from("fn Main(){");
    if resolved_projection {
        source.push_str("var q=(if true then {return;} else {return;});let x=q[0];q=[1];");
    }
    for index in 0..families {
        source.push_str(&format!("var a{index}=[];a{index}=[1];"));
    }
    source.push('}');
    source
}

fn allocation_traffic(families: usize, resolved_projection: bool) -> u64 {
    let source = source(families, resolved_projection);
    ALLOCATED.store(0, Ordering::Relaxed);
    let result = check_proposed("projection_scale.n", &source, false);
    let bytes = ALLOCATED.load(Ordering::Relaxed);
    assert!(result.is_ok(), "{result:?}");
    bytes
}

#[test]
#[ignore = "escala de asignaciones tras resolver proyección y sin proyecciones"]
fn resolved_projection_does_not_repartition_unrelated_pending_constraints() {
    for resolved_projection in [false, true] {
        let small = allocation_traffic(512, resolved_projection);
        let large = allocation_traffic(1536, resolved_projection);
        println!("projection={resolved_projection} allocations={small}/{large}");
        assert!(
            large * 10 <= small * 46,
            "familia {resolved_projection}: {small} -> {large}"
        );
    }
}
