use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};

use std::sync::Arc;

use state_game_runtime::persistent_vector::PersistentVector;

fn bench_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("push");

    group.bench_function("vec_push", |b| {
        b.iter(|| {
            let mut vec = Vec::new();

            for i in 0..10000 {
                vec.push(black_box(i));
            }

            black_box(vec);
        });
    });


    group.bench_function("persistent_vector_push", |b| {
        b.iter(|| {
            let mut vec = PersistentVector::new();

            for i in 0..10000 {
                vec = vec.push(Arc::new(black_box(i)));
            }

            black_box(vec);
        });
    });

    group.finish();
}


fn bench_get(c: &mut Criterion) {
    let mut pv = PersistentVector::new();

    for i in 0..100000 {
        pv = pv.push(Arc::new(i));
    }


    let mut vec = Vec::new();

    for i in 0..100000 {
        vec.push(i);
    }


    let mut group = c.benchmark_group("get");


    group.bench_function("vec_get", |b| {
        b.iter(|| {
            let mut sum = 0;

            for i in 0..100000 {
                sum += vec[black_box(i)];
            }

            black_box(sum);
        });
    });


    group.bench_function("persistent_vector_get", |b| {
        b.iter(|| {
            let mut sum = 0;

            for i in 0..100000 {
                sum += *pv.get(black_box(i)).unwrap();
            }

            black_box(sum);
        });
    });


    group.finish();
}


fn bench_iter(c: &mut Criterion) {
    let mut pv = PersistentVector::new();

    for i in 0..100000 {
        pv = pv.push(Arc::new(i));
    }


    c.bench_function(
        "persistent_vector_iter",
        |b| {
            b.iter(|| {
                let mut sum = 0;

                for value in pv.iter() {
                    sum += *value;
                }

                black_box(sum);
            });
        },
    );
}



fn bench_clone(c: &mut Criterion) {
    let mut pv = PersistentVector::new();

    for i in 0..100000 {
        pv = pv.push(Arc::new(i));
    }


    c.bench_function(
        "persistent_vector_clone",
        |b| {
            b.iter(|| {
                let cloned = pv.clone();

                black_box(cloned);
            });
        },
    );
}



fn bench_set(c: &mut Criterion) {
    let mut pv = PersistentVector::new();

    for i in 0..100000 {
        pv = pv.push(Arc::new(i));
    }


    c.bench_function(
        "persistent_vector_set",
        |b| {
            b.iter(|| {
                let result =
                    pv.set(
                        black_box(50000),
                        black_box(999),
                    )
                        .unwrap();

                black_box(result);
            });
        },
    );
}



criterion_group!(
    benches,
    bench_push,
    bench_get,
    bench_iter,
    bench_clone,
    bench_set
);

criterion_main!(benches);