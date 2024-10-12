use crate::vector_type::Vector;
use pgrx::opname;
use pgrx::pg_operator;

#[pg_operator(immutable, parallel_safe)]
#[opname(<=>)]
fn cosine_distance(left: Vector, right: Vector) -> f64 {
    let left = left.value.as_slice();
    let right = right.value.as_slice();
    acap::cos::cosine_distance(left, right)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::pg_test;
    use pgrx::Spi;

    #[pg_test]
    fn test_cosine_distance() {
        let distance =
            Spi::get_one::<f64>("select '[1, 2, 3]'::vector(3) <=> '[1, 2, 3]'::vector(3)")
                .unwrap()
                .unwrap();
        assert_eq!(distance, 0.0);

        // This kind of this tests should be done in integration tests, I think
        // sqllogictest will work well. Let's do that later.
        let first_id = Spi::get_one::<i32>(
            r#"
        with temp (id, embedding) as (
            select 1, '[1, 2, 3]'::vector(3) union all 
            select 2, '[4, 5, 6]'::vector(3)
        )
        select id from temp order by embedding <=> '[3, 1, 2]';
        "#,
        )
        .unwrap()
        .unwrap();

        assert_eq!(first_id, 2);
    }
}
