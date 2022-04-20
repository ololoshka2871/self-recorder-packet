pub(crate) trait AddSigned<T> {
    fn my_checked_add_signed(self, rhs: i32) -> Option<T>;
}

impl AddSigned<u32> for u32 {
    fn my_checked_add_signed(self, rhs: i32) -> Option<u32> {
        if rhs >= 0 {
            let rhs = rhs as u32;
            self.checked_add(rhs)
        } else {
            let mrhs = rhs.abs() as u32;
            self.checked_sub(mrhs)
        }
    }
}
