extern crate ratelimit_meter;
#[macro_use]
extern crate nonzero_ext;
extern crate test_utilities;

use ratelimit_meter::algorithms::Algorithm;

use ratelimit_meter::{NegativeMultiDecision, NonConformance, GCRA};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn accepts_first_cell() {
    let params =
        GCRA::params_from_constructor(nonzero!(5u32), nonzero!(1u32), Duration::from_secs(1))
            .unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();;
    assert_eq!(
        Ok(()),
        GCRA::test_and_update(&state, &params, Instant::now())
    );
}

#[test]
fn rejects_too_many() {
    let params =
        GCRA::params_from_constructor(nonzero!(1u32), nonzero!(1u32), Duration::from_secs(1))
            .unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();;
    let now = Instant::now();
    GCRA::test_and_update(&state, &params, now).unwrap();
    GCRA::test_and_update(&state, &params, now).unwrap();
    assert_ne!(
        Ok(()),
        GCRA::test_and_update(&state, &params, now),
        "{:?} {:?}",
        &state,
        &params
    );
}

#[test]
fn allows_after_interval() {
    let params =
        GCRA::params_from_constructor(nonzero!(1u32), nonzero!(1u32), Duration::from_secs(1))
            .unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();;
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    GCRA::test_and_update(&state, &params, now).unwrap();
    assert_eq!(Ok(()), GCRA::test_and_update(&state, &params, now + ms));
    assert_ne!(Ok(()), GCRA::test_and_update(&state, &params, now + ms * 2));
    // should be ok again in 1s:
    let next = now + Duration::from_secs(1);
    assert_eq!(Ok(()), GCRA::test_and_update(&state, &params, next));
}

#[test]
fn allows_n_after_interval() {
    let params =
        GCRA::params_from_constructor(nonzero!(2u32), nonzero!(1u32), Duration::from_secs(1))
            .unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();;
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    assert_eq!(Ok(()), GCRA::test_n_and_update(&state, &params, 2, now));
    assert!(!GCRA::test_n_and_update(&state, &params, 2, now + ms).is_ok());
    // should be ok again in 1.5s:
    let next = now + Duration::from_secs(1);
    assert_eq!(
        Ok(()),
        GCRA::test_n_and_update(&state, &params, 2, next),
        "now: {:?}",
        next
    );

    // should always accommodate 0 cells:
    assert_eq!(Ok(()), GCRA::test_n_and_update(&state, &params, 0, next));
}

#[test]
fn correctly_handles_per() {
    let ms = Duration::from_millis(1);
    let params = GCRA::params_from_constructor(nonzero!(1u32), nonzero!(1u32), ms * 20).unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();;
    let now = Instant::now();

    assert_eq!(Ok(()), GCRA::test_and_update(&state, &params, now));
    assert_eq!(Ok(()), GCRA::test_and_update(&state, &params, now + ms));
    assert!(!GCRA::test_and_update(&state, &params, now + ms * 10).is_ok());
    assert_eq!(
        Ok(()),
        GCRA::test_and_update(&state, &params, now + ms * 20)
    );
}

#[test]
fn never_allows_more_than_capacity() {
    let ms = Duration::from_millis(1);
    let params =
        GCRA::params_from_constructor(nonzero!(5u32), nonzero!(1u32), Duration::from_secs(1))
            .unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let now = Instant::now();

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert!(GCRA::test_n_and_update(&state, &params, 15, now).is_err());

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    assert!(GCRA::test_n_and_update(&state, &params, 15, now + (ms * 3 * 1000)).is_err());

    let result = GCRA::test_n_and_update(&state, &params, 15, now + (ms * 20 * 1000));
    match result {
        Err(NegativeMultiDecision::InsufficientCapacity(n)) => assert_eq!(n, 15),
        _ => panic!("Did not expect {:?}", result),
    }
}

#[test]
fn correct_wait_time() {
    // Bucket adding a new element per 200ms:
    let params =
        GCRA::params_from_constructor(nonzero!(5u32), nonzero!(1u32), Duration::from_secs(1))
            .unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let mut now = Instant::now();
    let ms = Duration::from_millis(1);
    let mut conforming = 0;
    for _i in 0..20 {
        now += ms;
        let res = GCRA::test_and_update(&state, &params, now);
        match res {
            Ok(()) => {
                conforming += 1;
            }
            Err(wait) => {
                now += wait.wait_time_from(now);
                assert_eq!(Ok(()), GCRA::test_and_update(&state, &params, now));
                conforming += 1;
            }
        }
    }
    assert_eq!(20, conforming);
}

#[test]
fn actual_threadsafety() {
    let params =
        GCRA::params_from_constructor(nonzero!(20u32), nonzero!(1u32), Duration::from_secs(1))
            .unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();

    let now = Instant::now();
    let ms = Duration::from_millis(1);
    let mut children = vec![];

    GCRA::test_and_update(&state, &params, now).unwrap();
    for _i in 0..20 {
        let state = state.clone();
        let params = params.clone();
        children.push(thread::spawn(move || {
            GCRA::test_and_update(&state, &params, now).unwrap();
        }));
    }
    for child in children {
        child.join().unwrap();
    }
    assert_ne!(Ok(()), GCRA::test_and_update(&state, &params, now + ms * 2));
    assert_eq!(
        Ok(()),
        GCRA::test_and_update(&state, &params, now + ms * 1000)
    );
}
