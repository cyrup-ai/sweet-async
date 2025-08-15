//! Extension trait for ergonomic size constants (e.g., 100.rows(), 50.items())

pub trait SizeExt {
    fn rows(self) -> usize;
    fn items(self) -> usize;
    fn lines(self) -> usize;
    fn records(self) -> usize;
    fn chunks(self) -> usize;
}

impl SizeExt for u64 {
    fn rows(self) -> usize {
        self as usize
    }
    fn items(self) -> usize {
        self as usize
    }
    fn lines(self) -> usize {
        self as usize
    }
    fn records(self) -> usize {
        self as usize
    }
    fn chunks(self) -> usize {
        self as usize
    }
}

impl SizeExt for usize {
    fn rows(self) -> usize {
        self
    }
    fn items(self) -> usize {
        self
    }
    fn lines(self) -> usize {
        self
    }
    fn records(self) -> usize {
        self
    }
    fn chunks(self) -> usize {
        self
    }
}
