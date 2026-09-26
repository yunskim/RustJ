//! Uninstrumented, same-process, end-to-end comparison with pinned libj.
//! Linux only. Both engines parse the same assignment on every iteration.
use rustj::Engine;
use std::{
    ffi::{CString, c_char, c_int, c_void},
    hint::black_box,
    time::Instant,
};

#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(name: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}
type Init = unsafe extern "C" fn() -> *mut c_void;
type Do = unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;
type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
struct J {
    library: *mut c_void,
    engine: *mut c_void,
    run: Do,
    free: Free,
}
impl J {
    fn new(path: &str) -> Self {
        let name = CString::new(path).unwrap();
        // SAFETY: pinned Linux x86-64 library implements these jlib.h signatures.
        unsafe {
            let library = dlopen(name.as_ptr(), 2);
            assert!(!library.is_null(), "cannot load {path}");
            let init = dlsym(library, c"JInit".as_ptr());
            let run = dlsym(library, c"JDo".as_ptr());
            let free = dlsym(library, c"JFree".as_ptr());
            assert!(!init.is_null() && !run.is_null() && !free.is_null());
            let init: Init = std::mem::transmute(init);
            let engine = init();
            assert!(!engine.is_null());
            Self {
                library,
                engine,
                run: std::mem::transmute::<*mut c_void, Do>(run),
                free: std::mem::transmute::<*mut c_void, Free>(free),
            }
        }
    }
    fn sentence(&self, text: &CString) {
        // SAFETY: engine and NUL-terminated input outlive this synchronous call.
        assert_eq!(unsafe { (self.run)(self.engine, text.as_ptr()) }, 0);
    }
}
impl Drop for J {
    fn drop(&mut self) {
        // SAFETY: no engine references or borrowed results survive this owner.
        unsafe {
            (self.free)(self.engine);
            dlclose(self.library);
        }
    }
}
fn main() {
    let path = std::env::var("J_LIBRARY").expect("set J_LIBRARY to pinned libj.so");
    let j = J::new(&path);
    let mut rust = Engine::new();
    if std::env::var_os("RUSTJ_MEMORY_PROBE").is_some() {
        memory_probe(&j, &mut rust);
        return;
    }
    for n in [16, 4096, 100_000, 1_000_000, 4_000_000] {
        for (name, expr, float) in [
            ("int_scalar_add", "r =: a + 2", false),
            ("int_vector_add", "r =: a + b", false),
            ("int_scalar_sub", "r =: a - 2", false),
            ("int_scalar_mul", "r =: a * 3", false),
            ("float_scalar_add", "r =: a + 2.5", true),
        ] {
            for s in [
                format!("a =: {}i. {n}", if float { "0.5 + " } else { "" }),
                format!("b =: i. {n}"),
            ] {
                rust.eval(&s).unwrap();
                j.sentence(&CString::new(s).unwrap());
            }
            let cexpr = CString::new(expr).unwrap();
            for _ in 0..4 {
                black_box(rust.eval(expr).unwrap());
                j.sentence(&cexpr);
            }
            let iters = if n <= 4096 {
                2000
            } else if n <= 100_000 {
                100
            } else {
                15
            };
            let mut rt = Vec::new();
            let mut ct = Vec::new();
            for round in 0..7 {
                for turn in 0..2 {
                    let start = Instant::now();
                    if (round + turn) % 2 == 0 {
                        for _ in 0..iters {
                            black_box(rust.eval(expr).unwrap());
                        }
                        rt.push(start.elapsed().as_secs_f64() * 1e6 / iters as f64);
                    } else {
                        for _ in 0..iters {
                            j.sentence(&cexpr);
                        }
                        ct.push(start.elapsed().as_secs_f64() * 1e6 / iters as f64);
                    }
                }
            }
            rt.sort_by(f64::total_cmp);
            ct.sort_by(f64::total_cmp);
            println!(
                "{{\"workload\":\"{name}\",\"n\":{n},\"iterations\":{iters},\"rust_us\":{:.3},\"c_us\":{:.3},\"rust_min_us\":{:.3},\"rust_max_us\":{:.3},\"c_min_us\":{:.3},\"c_max_us\":{:.3}}}",
                rt[3], ct[3], rt[0], rt[6], ct[0], ct[6]
            );
        }
    }
}

fn minor_faults() -> u64 {
    let s = std::fs::read_to_string("/proc/self/stat").unwrap();
    // Fields after the final ')' begin at field 3 (state); minflt is field 10.
    s.rsplit_once(')')
        .unwrap()
        .1
        .split_whitespace()
        .nth(7)
        .unwrap()
        .parse()
        .unwrap()
}
fn memory_probe(j: &J, rust: &mut Engine) {
    for n in [1_000_000, 4_000_000] {
        let s = format!("a =: i. {n}");
        rust.eval(&s).unwrap();
        j.sentence(&CString::new(s).unwrap());
        let expr = CString::new("r =: a + 2").unwrap();
        for _ in 0..5 {
            rust.eval("r =: a + 2").unwrap();
            j.sentence(&expr);
        }
        let before = minor_faults();
        for _ in 0..20 {
            black_box(rust.eval("r =: a + 2").unwrap());
        }
        let rust_faults = minor_faults() - before;
        let before = minor_faults();
        for _ in 0..20 {
            j.sentence(&expr);
        }
        let c_faults = minor_faults() - before;
        println!(
            "{{\"n\":{n},\"iterations\":20,\"rust_minor_faults\":{rust_faults},\"c_minor_faults\":{c_faults}}}"
        );
    }
}
