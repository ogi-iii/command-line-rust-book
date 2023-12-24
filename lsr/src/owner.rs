#[derive(Debug)]
pub enum Owner {
    User,
    Group,
    Other,
}

// メソッドの定義
impl Owner {
    pub fn masks(&self) -> [u32; 3] { // 8進数の値を3つ返す: rwx
        match self {
            Self::User => [0o400, 0o200, 0o100],
            Self::Group => [0o040, 0o020, 0o010],
            Self::Other => [0o004, 0o002, 0o001],
        }
    }
}
