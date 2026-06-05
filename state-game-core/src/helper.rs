pub fn try_until<T, E, F: FnMut() -> Result<T, E>>(mut to_try: F) -> Result<T, E> {
    const CAN_TRY_COUNT: usize = 5;
    for i in 1..=CAN_TRY_COUNT {
        match to_try() {
            Ok(v) => return Ok(v),
            Err(e) => {
                if i == CAN_TRY_COUNT {
                    return Err(e);
                }
            }
        }
    };

    unreachable!()
}