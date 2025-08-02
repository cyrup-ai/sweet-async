#[macro_export]
macro_rules! OK {
    ($pat:pat) => {
        Ok($pat)
    };
}

#[macro_export]
macro_rules! ERR {
    ($pat:pat) => {
        Err($pat)
    };
}
