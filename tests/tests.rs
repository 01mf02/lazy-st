use lazy_st::{lazy, Thunk};
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn evaluate_just_once() {
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();
    let val = lazy!({
        let mut data = counter.lock().unwrap();
        *data += 1;
    });
    *val;
    *val;
    assert_eq!(*counter_clone.lock().unwrap(), 1);
}

#[test]
fn multiple_closures() {
    let x = lazy!(0);
    let y = lazy!(1);
    let z = if true { x } else { y };
    assert_eq!(*z, 0);
}

#[test]
fn no_evaluate_if_not_accessed() {
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();
    let _val = lazy!({
        let mut data = counter.lock().unwrap();
        *data += 1;
    });
    assert_eq!(*counter_clone.lock().unwrap(), 0);
}

pub struct Dropper(Arc<Mutex<u64>>);

impl Drop for Dropper {
    fn drop(&mut self) {
        let Dropper(ref count) = *self;
        *count.lock().unwrap() += 1;
    }
}

#[test]
fn drop_internal_data_just_once() {
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();
    let result = thread::spawn(move || {
        let value = Dropper(counter_clone);
        let t = Thunk::<()>::new(move || {
            // Get a reference so value is captured.
            let _x = &value;

            panic!("Muahahahah")
        });
        t.force();
    })
    .join();

    match result {
        Err(_) => {
            assert_eq!(*counter.lock().unwrap(), 1);
        }
        _ => panic!("Unexpected success in spawned task."),
    }
}
